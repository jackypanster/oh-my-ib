# PRD — grid-tick

Stage: prd · feature: grid-tick · repo: jackypanster/oh-my-ib · branch: main
Author: cc. Design grilled & locked via `/think` (2026-07-07); five operator decisions recorded below
(D-CMD, D-CASH, D-FLAT, D-TARGET, D-MAXSH via AskUserQuestion + follow-ups). This is a NEW command
that ORCHESTRATES existing read + gated-write primitives; it introduces the repo's first *strategy*
(policy) surface. Paper-only in v1.

## Problem

The operator wants a deterministic (no LLM/agent) mean-reversion grid on a configurable set of stocks
(NVDA the running example): ladder **buy 100** each time price sits below the position's average cost by
a per-symbol % (default 2%), and **sell 100** each time price sits above avg cost by a per-symbol %
(default 2%). Today `omi` has the primitives (`positions` with `avg_cost`, `account` with cash/net-liq,
`orders`, gated `buy`/`sell --limit`, `cancel`) but nothing composes them into a scheduled reconcile
loop. Doing it by hand-spawning many one-shot `omi` processes per tick churns connections and scatters
the decision logic where it can't be tested.

## Goal

Add `omi grid-tick --config <path>`: **one invocation = one reconcile cycle**. On a SINGLE gateway
connection it reads account (cash, net-liq) + positions (qty, avg_cost) + open orders, runs a **pure,
offline-frozen planner** that emits a desired resting-order set per configured symbol, and reconciles
(cancel the stale, place the missing) via the existing gated write path. A cron/launchd entry supplies
the cadence (≥30–60s). No local state file — the resting orders + position IN IB are the state; a
crashed/restarted tick re-derives truth and continues (idempotent).

## Why resting limit orders make the rate-limit worry evaporate (design's load-bearing insight)

IB's matching engine watches the price; the script does NOT poll price at high frequency. Each tick does
one `account_updates` drain + one `all_open_orders` read + a few cancels/places ≈ 10–30 messages spread
over 1–2s. Against IB's binding limit (**50 msg/sec/connection**) a 60s tick averages <0.5 msg/s — orders
of magnitude of headroom. Historical-data pacing (strict) and streaming market-data lines are **not used**
at all. Bonus: the account has **delayed** market data, which would cripple a "poll price → decide" design
but is irrelevant to resting LIMIT orders (IB fills them off its own real-time book).

## Data flow

```
  cron/launchd (≥30–60s)
      │
      ▼
  omi grid-tick --config grid.toml   (ONE connection, stateless)
      │  reads (one account_updates drain + one all_open_orders):
      │    account{ total_cash, net_liquidation }
      │    positions: symbol → { qty, avg_cost }      ← the moving cost anchor (IB recomputes on each buy)
      │    open_orders: [{ order_id, symbol, action, limit_price, qty }]
      │
      │  plan_grid_tick(config, account, positions, open_orders) -> Vec<Action>   [PURE, FROZEN]
      │    per configured symbol → desired {buy?, sell?} → diff vs existing
      │
      ▼  execute on the SAME client (Cancels before Places), via the gated place path:
  cancel_with_client / place_core_with_client  ──►  IB Gateway ──► 撮合
                                                     (IB holds truth: resting orders + position)
```

## Code reality (verified against the repo — the design's load-bearing facts)

- **Notional cap is PAPER-EXEMPT.** `trade.rs:596` — "Paper (`cfg.port != LIVE_PORT` ⇒ is_live=false) is
  exempt"; `check_live_write_posture` returns `Ok` immediately when `!is_live` (`trade.rs:243`). So on
  paper `:4002`, 100-share NVDA (~$18.8k) orders place freely. On LIVE `:4001` the $500 cap
  (`DEFAULT_MAX_NOTIONAL`, `resolve_max_notional`) would REFUSE them → v1 is paper-only (D-TARGET).
- **One `account_updates` drain yields BOTH cash AND positions.** `account.rs` (`SummaryAccumulator`:
  NetLiquidation/TotalCashValue) and `positions.rs` (`position_row`: qty=`p.position`,
  avg_cost=`p.average_cost`) both drain the SAME `client.account_updates(account)` subscription, picking
  different `AccountUpdate` variants. The grid driver drains it ONCE and accumulates both — reuse both
  shapers so shapes never drift.
- **The single-connection pattern already exists.** `orders.rs` has `open_orders_with_client(&client, …)`
  as a `&Client` inner, with `orders(cfg)` a thin `connect`+delegate wrapper. `grid-tick` mirrors this:
  add `_with_client` inner helpers for the account_updates drain, for place, and for cancel; the existing
  `cfg`-taking `place()`/`cancel()` stay as thin wrappers. **This makes single-connection a mechanical
  refactor, not a risky one.**
- **Orders row exposes every planner input** (`orders.rs:37`): `order_id`, `symbol`, `action`
  (`"Buy"`/`"Sell"` via `{:?}`), `quantity`, `order_type`, `limit_price`.
- **Config plumbing already exists.** `Cargo.toml` depends on `toml = "0.8"` + `serde` derive;
  `config.rs` already does "CLI flag > `~/.config/oh-my-ib/config.toml` > default" with
  `std::fs::read_to_string` + toml parse. The grid config reuses this stack (format + parse are solved).
- **Write path is gated + LMT/notional-checked in `place_core`** (`trade.rs:582`, args `cfg, ctx,
  contract, order, ack`). It internally connects; the refactor extracts a `&Client` variant so the grid
  driver reuses the SAME gate/stamp logic (gating stays honest) on its single connection.

## Decisions (provenance-tagged; ✅ = human-confirmed via /think AskUserQuestion + follow-ups)

- **D-CMD — a new Rust subcommand `omi grid-tick`, single-tick (cron-scheduled), NOT a long-running
  daemon and NOT an external script.** ✅ (`/think` 2026-07-07). Single-stack (no new runtime — respects
  CLAUDE.md "never add a language without approval"); one connection per tick (operator explicitly valued
  "一次连接零 churn"); cron owns cadence. Consequence: MUST go through the full pipeline + a new ADR +
  write-containment decision (see For-arch).

- **D-RECONCILE — stateless reconcile-to-desired-state (K8s/Terraform style), NOT incremental.** ✅
  (`/think`). Each tick reads IB's full truth and converges; no local state file. Idempotent (orders
  already at desired price ⇒ empty plan ⇒ pure-read tick), crash-safe (restart re-derives), and slow
  reactions never do anything unsafe (a filled buy leaves the paired sell resting at a conservatively
  HIGHER price until the next tick re-prices it).

- **D-PLANNER — a PURE function `plan_grid_tick(config, account, positions, open_orders) -> Vec<Action>`
  is the frozen heart; the gateway driver is thin.** ✅ (design). The planner has NO client/gateway — it
  is 100% offline-testable and carries ~all the logic + all the frozen tests. `Action ∈ { Cancel(order_id)
  | Place{ symbol, side: Buy|Sell, qty, limit } }`. **Cancels are ordered before Places** in the output
  (free the book before re-placing; avoid opposite-side/duplicate conflicts).

- **D-THRESH — per-symbol thresholds off the live avg_cost anchor.** ✅ per-symbol %, default 2.0 each
  (operator: "每个被配置的股票都有上浮和下跌百分比配置,默认都是2%,可以不同"). For a held symbol
  (`qty>0`, anchor = `avg_cost`):
  - `sell_limit = round2(avg_cost × (1 + rise_pct/100))`, `sell_qty = min(lot, qty)` (min lets it reach
    true flat when an odd lot remains).
  - `buy_limit  = round2(avg_cost × (1 − drop_pct/100))`, `buy_qty = lot`.
  IB recomputes `average_cost` on each buy, so reading it fresh each tick gives the moving anchor for free
  — "按新持仓重下单" is automatic. (Sell does not change remaining-share avg_cost; only qty drops.)

- **D-CASH — buy suppressed when `total_cash < (cash_floor_pct/100) × net_liquidation`.** ✅ "净值的 50%"
  (`/think` AskUserQuestion). `cash_floor_pct` default 50, global. Evaluated fresh each tick (stateless —
  no persisted baseline). Soft floor: a fill can cross it, and the NEXT tick stops re-placing the buy — a
  ≤1-rung overshoot is accepted (v1 simplicity). Sell side is never cash-gated.

- **D-MAXSH — per-symbol hard position ceiling `max_shares`, default 300.** ✅ (`/think` follow-up:
  "max shares 默认 300 股"). Buy is placed only if `qty + lot ≤ max_shares` (**never exceed** — the strict
  form, so a non-lot-multiple `max_shares` still can't be overshot; e.g. 250 stops laddering at 200). This
  is the only hard brake on the mean-reversion blow-up (see Fragile assumption). `max_shares` optional per
  symbol; unset ⇒ no ceiling (cash floor is then the sole brake).

- **D-FLAT — a configured symbol at `qty == 0` has no cost anchor ⇒ idle + cancel lingering orders.** ✅
  "闲置,等手动重新播种" (`/think` AskUserQuestion). desired set is empty; the planner emits Cancels for any
  still-resting orders on that symbol (a clean stop, so a stale buy can't silently re-establish a
  position). The operator manually `omi buy`s to re-seed the anchor and restart the ladder. Matches the
  spec "卖到没有持仓为止".

- **D-TARGET — v1 paper `:4002` ONLY.** ✅ (`/think` AskUserQuestion). Paper exempts the notional cap so
  100-share orders work as-is with zero live risk; validate the loop first. LIVE is deferred: it needs
  `OMI_MAX_NOTIONAL` raised to ~$25k, which would neuter the fat-finger cap for ALL live orders — an
  explicit separate decision, not v1.

- **D-LMT — LMT/DAY orders only; no MKT, no `--outside-rth` in v1.** ✅ (design). The grid needs limit
  prices by construction; MKT is excluded (also keeps it live-safe if promoted later). `outside_rth` is
  deferred to a follow-up card (a grid works fine in RTH; wiring the just-shipped flag through the config
  adds surface for marginal v1 value). Owner: cc, post-v1.

- **D-RECONCILE-TOL — keep an existing order iff same side, same qty, and `|actual.limit − desired.limit|
  ≤ $0.005`; else Cancel+Place.** ✅ (design). Prices `round2` (NVDA min tick $0.01; v1 assumes ≥$1
  names / 2-decimal ticks). The tolerance stops churn from float noise. Any >1 same-side existing order on
  a symbol, or a side not in `desired`, is Cancelled.

## The pure planner contract (this is what `pipeline-task` freezes)

```
Config        { lot: u32 = 100, cash_floor_pct: f64 = 50.0, symbols: [SymbolCfg] }
SymbolCfg     { name: String, drop_pct: f64 = 2.0, rise_pct: f64 = 2.0, max_shares: Option<u32> = Some(300) }
AccountSnap   { total_cash: f64, net_liquidation: f64 }
Position      { qty: f64, avg_cost: f64 }                      // keyed by symbol, from position_row
OpenOrder     { order_id: i32, symbol: String, side: Buy|Sell, limit: f64, qty: f64 }
Action        = Cancel(order_id: i32) | Place{ symbol, side, qty, limit }

plan_grid_tick(cfg, acct, positions: map<sym,Position>, open_orders: [OpenOrder]) -> [Action]

for each s in cfg.symbols:
    pos      = positions[s.name]  (qty=0 if absent)
    existing = open_orders where symbol == s.name
    desired  = {}   // 0..2 Places
    if pos.qty > 0:                                            // has an anchor
        desired += Place{ s.name, Sell, min(cfg.lot, pos.qty), round2(pos.avg_cost*(1+s.rise_pct/100)) }
        buy_ok = acct.total_cash >= (cfg.cash_floor_pct/100)*acct.net_liquidation
                 && (s.max_shares is None || pos.qty + cfg.lot <= s.max_shares)
        if buy_ok:
            desired += Place{ s.name, Buy, cfg.lot, round2(pos.avg_cost*(1-s.drop_pct/100)) }
    // reconcile existing → desired, per side:
    //   matching existing (same side & qty & |Δlimit|<=0.005)  → keep (no Action)
    //   desired side w/o match                                 → Place(desired)
    //   existing side not desired, or duplicate, or qty==0 all → Cancel(order_id)
emit all Cancels, then all Places.   // symbols NOT in cfg are never touched
```

Common steady state between fills ⇒ every desired order already matches ⇒ **empty plan ⇒ zero writes**.

## Scope

- **IN** `src/cli.rs`: new `GridTick` subcommand variant + args (`--config <path>`, optional
  `--dry-run` to print the plan without executing — cheap, high-value for operator trust).
- **IN** a new module for the **pure planner** + the config structs (offline, frozen) — placement (new
  `src/grid.rs` or `src/ib/grid.rs`) is an arch decision (see For-arch / write-containment).
- **IN** `src/ib/*`: `_with_client` inner helpers — an account_updates drain that accumulates
  `SummaryAccumulator` + `Vec<position_row>` in one pass; `cancel_with_client`; `place_core_with_client`
  (thin `&Client` extractions; existing `cfg`-wrappers preserved). The gateway **driver** `grid_tick(cfg,
  cfg_path)` orchestrating read → plan → execute.
- **IN** config parsing: a `[grid]` surface (dedicated `--config grid.toml`, OR a `[grid]` table in the
  existing `~/.config/oh-my-ib/config.toml`) — arch picks; recommend a dedicated file for op/versioning
  separation, reusing the toml+serde stack.
- **IN** a new ADR (0033) recording the strategy/policy layer + write-containment extension + the
  reconcile model; `CONTEXT.md` glossary ("grid tick", "reconcile", "cost anchor", "cash floor",
  "position ceiling").
- **IN** NEW frozen spec `tests/grid_tick.rs`.
- **OUT** (non-scope): long-running daemon / internal scheduler (cron owns cadence); LIVE `:4001`
  (D-TARGET); MKT and `outside_rth` (D-LMT); multi-account (single resolved account); per-order ownership
  tagging via `orderRef` (v1 grid owns ALL orders on configured symbols — see gotcha); sub-$1 fine-tick
  rounding; partial-fill special handling (next tick's reconcile absorbs it); a hard stop-loss price (only
  `max_shares` in v1 — see Fragile assumption); any change to existing risk gates
  (`require_live_write_gate` / `check_live_write_posture` / `combo_live_max_risk` all untouched).

## Success criteria (acceptance)

1. **Planner, held symbol (offline, FROZEN):** `qty>0`, cash above floor, `qty+lot ≤ max_shares` ⇒ plan
   contains exactly one Buy @ `round2(avg_cost*(1-drop%))` qty=lot AND one Sell @
   `round2(avg_cost*(1+rise%))` qty=`min(lot,qty)`, when no existing orders. [frozen]
2. **Cash floor (FROZEN):** `total_cash < 0.5*net_liq` ⇒ NO Buy in the plan; the Sell still present. [frozen]
3. **max_shares ceiling (FROZEN):** `qty+lot > max_shares` (e.g. qty=300, lot=100, max=300) ⇒ NO Buy;
   qty=200,max=250 ⇒ NO Buy (strict, never exceed); qty=200,max=300 ⇒ Buy present. [frozen]
4. **Flat idle (FROZEN):** `qty==0` with a lingering resting order on that symbol ⇒ plan = Cancel(that
   order), no Place; `qty==0` with no orders ⇒ empty plan. [frozen]
5. **Reconcile idempotence (FROZEN):** existing orders already at desired side/qty/price (±$0.005) ⇒
   empty plan (zero Actions). A price drift beyond tolerance ⇒ Cancel(old)+Place(new), Cancels first. [frozen]
6. **Per-symbol independence + unconfigured untouched (FROZEN):** two configured symbols with different
   drop/rise% each get their own pair; an open order on an UNconfigured symbol yields no Action. [frozen]
7. **Config parse (FROZEN):** a valid toml parses to Config with defaults applied (lot=100,
   cash_floor_pct=50, drop=rise=2.0, max_shares=300); a malformed/negative-% config ⇒ `code="config"`
   (exit 5), offline. [frozen]
8. **Paper acceptance (operator, `:4002`):** seed a position (`omi buy NVDA 100 --limit <near>` → fill);
   `omi grid-tick --config grid.toml --dry-run` prints the intended Buy@-2%/Sell@+2%; a real
   `grid-tick` places both (`omi orders` shows them); re-run ⇒ idempotent (no new orders); manually
   cancel one, re-run ⇒ re-placed; sell to flat, re-run ⇒ lingering order cancelled + symbol idle. [operator paper]
9. **Guardrails intact:** grid-tick does not touch the live gate, notional cap, or combo breaker; existing
   frozen suites stay GREEN and byte-identical. [freeze gate + read]
10. `cargo build` · full `cargo test` · `cargo clippy --all-targets -- -D warnings` green. [verify]

## Fragile assumption (premise collapse — stated per /think)

**The strategy assumes price mean-reverts around the cost basis.** In a sustained downtrend the planner
buys 100 each −drop% rung, averaging down, while the sell condition (price > cost+rise%) never triggers,
until `max_shares` (D-MAXSH) or the cash floor (D-CASH) halts buying — leaving a large underwater
position with **no stop-loss exit** (out of scope). `max_shares=300` bounds the worst case to 300 shares;
the operator was told and accepted this. This is a strategy risk, not a code bug — surfaced, not hidden.

Second (technical) premise: the `_with_client` single-connection refactor stays mechanical. If
`place_core` turns out too coupled to `connect(cfg)` to split cleanly, the fallback is the grid driver
calling the existing `cfg`-wrappers (several connects/tick) — still fine on paper @60s, just not "one
connection". The established `open_orders_with_client` precedent makes this low-risk.

## Gotchas (project-specific traps the next nodes MUST know)

- **Grid owns ALL orders on a configured symbol.** The reconcile will Cancel any resting order on a
  configured symbol that doesn't match `desired` — including one the operator placed by hand. While a
  symbol is under grid management, don't place manual orders on it. (v2 could tag via `orderRef`; out of
  scope now — document loudly.)
- **Never touch unconfigured symbols.** The planner iterates `cfg.symbols` only; positions/orders for any
  other symbol are invisible to it. Preserve this — it's the blast-radius guard.
- **Write-containment (ADR 0017) collision.** Order-placement code lives ONLY in `trade.rs`. The grid
  driver *causes* writes (it calls place/cancel). Arch MUST resolve whether the planner+driver live in a
  new module blessed by ADR 0033 (recommended) or inside `trade.rs`, and keep the actual place/cancel
  going through the gated `place_core`/`cancel` logic (gate + stamp + notional) — do NOT re-implement the
  write.
- **Single connection = one `account_updates` drain for BOTH cash and positions** — don't open two.
- **Cost anchor is `p.average_cost` from the portfolio stream** (`positions.rs:52`), NOT last price. For
  STK it is per-share avg cost. Round limits to 2 decimals.
- **Cash floor is soft / per-tick** — a fill can dip cash below the floor; the next tick stops the buy.
  Don't try to make it hard (would need pre-trade cash reservation = state).
- **`--dry-run` must NOT connect for writes** but DOES need reads to compute the plan — it prints Actions
  and exits without executing them (operator trust + safe first run).

## Verify

`cargo build` · `cargo test` (new `tests/grid_tick.rs` red→green; all prior suites green) ·
`cargo clippy --all-targets -- -D warnings`. Operator paper: criterion 8 on `:4002` (no gateway needed
for the frozen offline planner + config-parse checks).

## For arch (next stage — resolve these, don't re-open the locked D-decisions)

1. **Module placement + write-containment (the #1 arch call).** New `src/grid.rs` (pure planner + config
   structs + driver) vs splitting planner into a pure module and keeping the driver near `trade.rs`.
   Author ADR 0033: strategy/policy layer boundary, the reconcile model, and the containment extension
   (grid orchestrates but the gated place/cancel primitives stay authoritative). Recommend: pure planner
   + config in a new module (freezable), driver reuses `place_core_with_client`/`cancel_with_client`.
2. **Exact `_with_client` refactor surface.** Confirm `place_core` and `cancel` split cleanly into
   `&Client` inners mirroring `open_orders_with_client`; name them; keep the `cfg`-wrappers byte-behavior.
   Confirm one `account_updates` drain can feed both `SummaryAccumulator` and `Vec<position_row>`.
3. **Config shape + location.** Dedicated `--config <path>` file vs `[grid]` in the existing
   `~/.config/oh-my-ib/config.toml`; the serde structs + defaults (serde `#[serde(default)]`); validation
   (positive %, non-empty symbols, floor in (0,100]) → `code="config"`.
4. **Action → execution mapping.** How each `Place`/`Cancel` maps onto the gated path on the shared client
   (Buy/Sell via `build_stk_order` + `place_core_with_client`; Cancel via `cancel_with_client`), and the
   command's JSON output shape (actions taken / dry-run plan / no-op) for auditable cron logs.
5. Confirm `CONTEXT.md` terms and that no existing risk seam changes (D-CASH/D-MAXSH are grid-local, not
   the $500 cap).
