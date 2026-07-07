# CONTEXT — grid-tick (domain language)

Ubiquitous terms this feature adds/sharpens. Ground for the cold task/impl/review nodes.

- **grid-tick** — `omi grid-tick --config <path>`: ONE reconcile cycle per invocation. Not a loop, not a
  daemon — cron/launchd supplies the cadence (≥30–60s). One `connect` per tick does all reads+writes then
  disconnects (reaffirms ADR 0003 stateless-connect-per-command).
- **reconcile-to-desired-state** — the control model (K8s/Terraform style): each tick reads IB's full
  truth, the pure planner computes the DESIRED resting-order set, and the driver emits only the diff
  (cancel the stale, place the missing). Idempotent (already-correct ⇒ empty plan ⇒ zero writes),
  crash-safe (state lives in IB — resting orders + position — never in a local file), and safe under slow
  reaction (a filled buy leaves the paired sell resting at a conservatively HIGHER price until re-priced).
- **cost anchor** — the position's `avg_cost` (`p.average_cost` from the `account_updates` PortfolioValue
  stream, `positions.rs:52`), per-share for STK. The grid's thresholds hang off it. IB recomputes it on
  each buy, so reading it fresh each tick gives a MOVING anchor for free ("按新持仓重下单" is automatic).
  A symbol with `qty == 0` has NO anchor ⇒ the grid idles it (see flat-idle).
- **drop_pct / rise_pct** — per-symbol thresholds (default 2.0 each, configurable per symbol). Buy limit =
  `round2(avg_cost*(1 - drop_pct/100))`; sell limit = `round2(avg_cost*(1 + rise_pct/100))`. Mean
  reversion: buy the dip below cost, sell the rip above.
- **lot** — shares per rung (global, default 100). Buy is always `lot`; sell is `min(lot, qty)` so the
  ladder can reach true flat when an odd lot remains.
- **cash floor** — buy is suppressed when `total_cash < (cash_floor_pct/100) * net_liquidation`
  (default 50% of net-liq). SOFT + per-tick (no persisted baseline): a fill may dip cash below it; the
  next tick just stops re-placing the buy (≤1-rung overshoot accepted). Sell is never cash-gated.
- **max_shares** — per-symbol HARD position ceiling (default 300). Buy placed only if
  `qty + lot <= max_shares` (STRICT never-exceed; a non-lot-multiple ceiling can't be overshot).
  `max_shares < lot` = sell-only (legal). This is the ONLY hard brake on the mean-reversion blow-up (a
  code brake, not a strategy cure — there is no stop-loss in v1).
- **flat-idle** — a configured symbol at `qty == 0`: desired set empty ⇒ the planner CANCELS any lingering
  resting order on it (a clean stop, so a stale buy can't silently rebuild a position) and places nothing.
  The operator manually `omi buy`s to re-seed the anchor and restart the ladder ("卖到没有持仓为止").
- **the pure planner** — `plan_grid_tick(cfg, acct, positions, open) -> Vec<Action>` in `src/grid.rs`: no
  client, no I/O ⇒ 100% offline-frozen. The feature's heart and its entire frozen surface
  (`tests/grid_tick.rs`). `Action = Cancel{order_id} | Place{symbol,side,qty,limit}`; **Cancels ordered
  before Places** in the output.
- **the driver** — `grid_tick(cfg,args)` in `src/ib/grid.rs`: the thin gateway layer (connect, one
  `account_updates` drain for cash+positions, `all_open_orders`, call the planner, execute, shape JSON).
  Review-by-reading + paper acceptance; NOT frozen.
- **grid owns the symbol (blast radius)** — the reconcile manages ALL orders on a CONFIGURED symbol: it
  will cancel a manual order you placed on one. Symbols NOT in the config are never touched — the planner
  iterates `cfg.symbols` only. (v2 could tag ownership via `orderRef`; out of scope.)
- **write containment (grid ⊂ ADR 0017)** — grid contains NO raw `place_order`/`cancel_order`; it composes
  the trade.rs choke points: `build_stk_order` (4-arg, unchanged), `place_with_client` (the ADR 0024
  account-stamping choke point, made `pub(crate)`), and a new `cancel_with_client`. So the raw write
  symbols still live ONLY in `trade.rs` — grid is a sanctioned consumer, like `place_core`/`option_combo`.
- **paper-only (v1)** — `grid_tick` refuses when `cfg.port == LIVE_PORT` (`code="config"`, exit 5,
  offline). 100 shares × ~$188 ≈ $18.8k >> the $500 live notional cap, so live would refuse every order;
  live is a separate future decision. Grid adds NO new order type/TIF — it's DAY LMT STK, the existing v1
  write surface (ADR 0017 §5); the grid is emulated by re-placing individual orders each tick, not
  IB-native brackets/GTC.
- **--dry-run** — reads + plans + prints the intended Actions with `dry_run:true`, executes NOTHING (still
  connects for the reads; issues zero writes). Operator trust + safe first run.
- **round2** — `(x*100.0).round()/100.0`, applied to computed limit prices (NVDA min tick $0.01; v1
  assumes ≥$1 / 2-decimal names). Reconcile match tolerance is `|Δlimit| <= 0.005`.

Unchanged domain (do NOT touch): `build_stk_order` body/signature, `shape_order_ack`, `place_core`,
`preview_stk_option`, `place`, `cancel`'s public signature/behavior, `require_live_write_gate`,
`check_live_write_posture`, `resolve_max_notional`, `combo_live_max_risk`, all `option-*` verbs,
`src/config.rs`. The grid-local brakes (cash floor, max_shares) are NOT the $500 notional cap.
