# ADR 0033 — grid-tick (cron-driven mean-reversion grid, single-connection reconcile)

Status: accepted · 2026-07-07 · feature: grid-tick · extends ADR 0017 (write containment), reaffirms
ADR 0003 (stateless connect-per-command); grid-local risk brakes, does NOT touch ADR 0030/0031 gates.

## Context

The operator wants a deterministic (no LLM/agent) mean-reversion grid over a configurable set of stocks:
ladder **buy `lot`** below the position's average cost by a per-symbol `drop_pct` (default 2%), **sell
`lot`** above avg cost by `rise_pct` (default 2%), bounded by cash and a position ceiling. The `omi`
primitives exist (`positions` avg_cost, `account` cash/net-liq, `orders`, gated `buy`/`sell --limit`,
`cancel`) but nothing composes them into a scheduled loop. Hand-spawning many one-shot `omi` processes per
tick churns connections and scatters the decision logic where it can't be tested. Design grilled & locked
via `/think` (2026-07-07); five operator decisions below.

## Decision — a new `omi grid-tick` command: pure planner + thin single-connection driver

`omi grid-tick --config <path>` runs **one reconcile cycle per invocation** (cron/launchd owns cadence).
On a SINGLE connection it reads account+positions+open-orders, runs a **pure, offline-frozen planner**
`plan_grid_tick`, and reconciles the resting-order set via the existing trade.rs write choke points. State
lives in IB (resting orders + position) — no local state file; a crashed/restarted tick re-derives truth
(idempotent). Reviewable code splits in two:

- **`src/grid.rs` — PURE policy (frozen).** Config structs + parse/validate, `plan_grid_tick`, the
  `Action` type, and the lightweight input structs. No `crate::ib`, no client → 100% offline-testable.
  This is where ~all logic and all frozen tests live (`tests/grid_tick.rs`).
- **`src/ib/grid.rs` — the gateway DRIVER (review-by-reading + paper acceptance).** `grid_tick(cfg,args)`:
  load config, refuse live, connect once, read → build the pure inputs, call `crate::grid::plan_grid_tick`,
  execute the Actions on the shared client, shape the JSON envelope.

### D-CMD — new Rust subcommand, single-tick, NOT a daemon or external script

Single-stack (no new runtime — respects the "never add a language without approval" rule); one connection
per tick (operator explicitly valued "一次连接零 churn"); cron owns cadence. Rejected: an external
Python/bash orchestrator spawning many `omi` processes (connection churn, untestable scattered logic); a
long-running daemon (holds a connection, needs crash-recovery + an internal scheduler — more surface for
no benefit when cron already schedules). **Reaffirms ADR 0003** (stateless connect-per-command): a tick is
ONE `connect` doing all its reads+writes then disconnecting — more aligned with 0003 than N processes.

### D-CONTAINMENT — grid composes trade.rs choke points; raw writes stay contained (extends ADR 0017)

ADR 0017 §1: all raw `place_order`/`cancel_order` calls live in ONE module (`trade.rs`), grep-enforced.
**grid.rs / ib/grid.rs contain NO raw ibapi write call.** The driver places via the EXISTING
`place_with_client` (`trade.rs:463`, the ADR 0024 account-stamping choke point — already called directly
by `option_combo`) and a new sibling `cancel_with_client`, and builds orders via `build_stk_order`
(unchanged, 4-arg frozen). So the raw write symbols still appear ONLY in `trade.rs`; grid is a new
**sanctioned consumer** of the choke points, exactly like `place_core` and `option_combo`. ADR 0017's
grep invariant holds verbatim (the "no read command imports trade.rs" clause is about READ commands; grid
is a write orchestrator, allowed). grid-tick adds **no new order type or TIF** — it composes the existing
v1 write surface (DAY LMT STK, ADR 0017 §5), so §5's "no GTC/stops/brackets" is not violated: the grid is
emulated by individual DAY LMT orders re-placed each tick, not IB-native brackets/GTC.

### D-TARGET — paper `:4002` ONLY in v1; grid-tick hard-refuses live

`grid_tick` refuses at the top when `cfg.port == LIVE_PORT` ⇒ `code="config"` (exit 5), offline. Rationale:
100 shares × ~$188 ≈ $18.8k >> the $500 live notional cap (`DEFAULT_MAX_NOTIONAL`), which would REFUSE
every grid order on live; raising `OMI_MAX_NOTIONAL` to ~$25k to permit it would neuter the fat-finger cap
for ALL live orders — an explicit, separate operator decision, not v1. Because grid can't reach live, it
needs none of the live gate / notional / posture machinery (paper is exempt anyway, `trade.rs:596`); the
existing gates (`require_live_write_gate`, `check_live_write_posture`, `combo_live_max_risk`) are
**untouched**. Live promotion is a future ADR.

### D-PLANNER — a pure `plan_grid_tick` is the frozen heart; reconcile-to-desired-state

```
plan_grid_tick(cfg: &GridConfig, acct: &AccountSnap,
               positions: &Map<sym, PositionLite>, open: &[OpenOrderLite]) -> Vec<Action>
Action = Cancel { order_id: i32 } | Place { symbol, side: Side, qty: f64, limit: f64 }
```

Per configured symbol (symbols NOT in the config are NEVER touched — the blast-radius guard):

- `qty == 0` (no cost anchor, D-FLAT) ⇒ desired empty ⇒ emit `Cancel` for every still-resting order on
  that symbol (a clean stop; a stale buy can't silently re-establish a position). Operator manually
  `omi buy`s to re-seed. Matches "卖到没有持仓为止".
- `qty > 0` (anchor = `avg_cost`, IB-recomputed on each buy ⇒ moving anchor for free):
  - `desired_sell = Place{sym, Sell, min(lot, qty), round2(avg_cost*(1+rise_pct/100))}`.
  - `buy_ok = acct.total_cash >= (cash_floor_pct/100)*acct.net_liquidation && qty + lot <= max_shares`.
    If `buy_ok`: `desired_buy = Place{sym, Buy, lot, round2(avg_cost*(1-drop_pct/100))}`.
- Reconcile existing vs desired, per side: an existing order matches iff **same side ∧ same qty ∧
  `|Δlimit| <= 0.005`** ⇒ keep (no Action); a desired side with no match ⇒ Place; an existing order whose
  side is not desired, or a duplicate, or any order when flat ⇒ Cancel.

Output: **all Cancels first, then all Places** (free the book before re-placing). Steady state between
fills ⇒ every desired order already matches ⇒ **empty plan ⇒ a pure-read tick, zero writes**.

### D-CASH — buy suppressed when `total_cash < (cash_floor_pct/100) * net_liquidation`

`cash_floor_pct` default 50, global (operator: "净值的 50%"). Evaluated fresh each tick (stateless — no
persisted baseline). **Soft floor**: a fill can dip cash below it; the NEXT tick simply stops re-placing
the buy (≤1-rung overshoot accepted for v1 simplicity — a hard pre-trade reservation would require state).
The sell side is never cash-gated.

### D-MAXSH — per-symbol hard ceiling `max_shares`, default 300, strict never-exceed

`buy_ok` requires `qty + lot <= max_shares` (NOT `qty < max_shares`), so a non-lot-multiple ceiling can't
be overshot (e.g. `max_shares=250, lot=100` stops laddering at 200). Default 300 (operator-set). This is
the only hard brake on the mean-reversion blow-up (see Consequences). `max_shares < lot` is legal and
means "sell-only, never ladder" (documented, not an error).

## Consequences

- `omi grid-tick --config grid.toml` on `:4002`: reads truth, ladders DAY-LMT buys below cost / sells
  above, bounded by cash floor + `max_shares`, idempotent between fills. `--dry-run` prints the plan
  without executing (still reads; issues no writes) — operator trust + safe first run.
- Rate-safe by construction: resting LIMIT orders offload price-watching to IB's matching engine, so a
  tick polls state ONCE (no price polling); ~10–30 msgs/tick vs IB's 50 msg/s/connection limit ⇒ <0.5
  msg/s at a 60s cadence. Immune to the account's delayed market data (IB fills off its own book).
- `build_stk_order`, `shape_order_ack`, `place_core`, `cancel` (public API), the live gate, the notional
  cap, the combo breaker, all option verbs, and every existing frozen test are unchanged. New crate-visible
  surface: `place_with_client` → `pub(crate)`; a new `pub(crate) cancel_with_client`; a `pub(crate)`
  account+positions read helper; `pub mod grid`.
- **Strategy risk (surfaced, not hidden):** the grid assumes price mean-reverts around cost. In a
  sustained downtrend it averages down while the sell never triggers, until `max_shares` or the cash floor
  halts buying — leaving an underwater position with **no stop-loss** (out of scope). `max_shares=300`
  bounds the worst case to 300 shares; the operator was told and accepted. A code brake (`max_shares`), not
  a strategy cure.
- **Blast radius:** the reconcile owns ALL orders on a configured symbol — it will cancel a manual order
  placed on one. Symbols not in the config are never touched. (v2 could tag ownership via `orderRef`.)

## Freeze coverage

- **FROZEN** (`tests/grid_tick.rs`, all offline): `plan_grid_tick` across held-symbol (buy+sell pair),
  cash-floor-suppresses-buy, max_shares-strict-ceiling, flat-idle-cancels-lingering, reconcile idempotence
  (already-matched ⇒ empty; drift ⇒ cancel-then-place), per-symbol independence + unconfigured-untouched;
  and `GridConfig` parse (valid toml ⇒ defaults applied; malformed/negative-% ⇒ `code="config"`).
- **REVIEW-BY-READING** (not frozen): the `ib/grid.rs` driver — single `account_updates` drain feeding
  cash+positions; `all_open_orders` read; Action→execution mapping on the shared client (Cancels-first,
  stop-on-first-error, ADR 0017 no-blind-retry); the live-refusal guard; the JSON envelope; the `pub(crate)`
  extractions leaving `cancel`/`place`/`build_stk_order` behavior byte-identical + prior suites green.
- **OPERATOR ACCEPTANCE** (paper `:4002`): seed a position; `--dry-run` shows the intended pair; a real
  tick places both (visible in `omi orders`); re-run ⇒ idempotent; cancel one + re-run ⇒ re-placed; sell to
  flat + re-run ⇒ lingering cancelled + idle. Live = deferred (separate ADR).

## Alternatives rejected

- **External Python/bash orchestrator over one-shot `omi`.** Connection churn per tick; strategy logic
  scattered in an untestable shell; a second runtime in a Rust repo (D-CMD).
- **Long-running daemon holding a connection.** Needs an internal scheduler + crash recovery for no gain
  when cron schedules; contradicts ADR 0003 stateless-per-command (D-CMD).
- **Poll price → decide → place (no resting orders).** Crippled by the account's delayed market data, and
  high-frequency price polling risks IB pacing; resting LIMITs sidestep both (Consequences).
- **Put the planner in `trade.rs`.** trade.rs is already large and the planner is pure policy; a separate
  `src/grid.rs` keeps the freezable policy isolated while the raw writes stay contained in trade.rs
  (D-CONTAINMENT).
- **Live in v1 by raising `OMI_MAX_NOTIONAL`.** Neuters the fat-finger cap for all live orders; deferred to
  an explicit decision (D-TARGET).
- **Hard cash reservation / persisted baseline.** Requires local state, breaking the stateless-reconcile
  property; the soft per-tick floor with a ≤1-rung overshoot is accepted (D-CASH).
