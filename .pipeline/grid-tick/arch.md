# arch — grid-tick

Stage: arch · feature: grid-tick · author: cc (grill-with-docs against the codebase).
Binding decisions in ADR 0033. This file pins the module boundaries + the exact type signatures so
`pipeline-task` can write red tests without re-deciding. The five locked D-decisions (ADR 0033) are NOT
re-opened here.

## Chosen shape — pure planner (frozen) + thin gateway driver (review-by-reading)

```
                         cron/launchd (≥30–60s)
                                │
                                ▼
   src/cli.rs  GridTick{ config: PathBuf, dry_run: bool }
                                │  main.rs dispatch → crate::ib::grid_tick(&cfg, &args)
                                ▼
   ┌─────────────────────────  src/ib/grid.rs  (DRIVER — gateway layer, NOT frozen)  ─────────────────────┐
   │ grid_tick(cfg, args):                                                                                 │
   │   1. GridConfig::load(&args.config)?          ── crate::grid  (pure parse+validate)                   │
   │   2. if cfg.port == LIVE_PORT → AppError::config (paper-only, D-TARGET), offline                      │
   │   3. client = ib::connect(cfg)?ㅤaccount = ib::resolve_account(&client, cfg)?      (ONE connection)   │
   │   4. (snap, positions) = read_account_positions(&client, cfg.account)?  ── ONE account_updates drain  │
   │   5. open = open_orders_with_client(&client, cfg.account, "grid-tick")? → Vec<OpenOrderLite>          │
   │   6. actions = crate::grid::plan_grid_tick(&gcfg, &snap, &positions, &open)   ── PURE, FROZEN          │
   │   7. if args.dry_run → shape(actions, dry_run=true), return (NO writes)                                │
   │      else execute in order (Cancels first): Cancel→cancel_with_client; Place→build_stk_order          │
   │           + place_with_client(&client,…,&account,ack);  stop on first error (ADR 0017 no retry)        │
   │   8. return JSON envelope { account, dry_run, planned, actions:[…results…] }                           │
   └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
                                │ calls (never raw place_order/cancel_order — those stay in trade.rs)
                                ▼
   ┌────────  src/grid.rs  (PURE policy — FROZEN, no crate::ib, no client, offline-testable)  ────────┐
   │ GridConfig / SymbolCfg  (serde + defaults)   GridConfig::load(path)->Result<_,AppError>(config)  │
   │ plan_grid_tick(&GridConfig,&AccountSnap,&Map<String,PositionLite>,&[OpenOrderLite]) -> Vec<Action>│
   │ Action = Cancel{order_id} | Place{symbol,side,qty,limit}    Side = Buy|Sell                       │
   │ AccountSnap{total_cash,net_liquidation}  PositionLite{qty,avg_cost}  OpenOrderLite{order_id,      │
   │   symbol,side,limit,qty}                                                                          │
   └───────────────────────────────────────────────────────────────────────────────────────────────────┘
```

No cycle: `src/grid.rs` (pure) depends on nothing repo-side but `crate::error`; `src/ib/grid.rs` depends
on `crate::grid` (types + planner) + `crate::ib` internals. One direction only.

## Component boundaries + write-set (for the task cards)

**New files**
- `src/grid.rs` — pure. `pub mod grid;` in `src/lib.rs`. Contains: config structs + `GridConfig::load` +
  validation; `plan_grid_tick`; `Action`/`Side`/`AccountSnap`/`PositionLite`/`OpenOrderLite`. **FROZEN**
  surface (tested by `tests/grid_tick.rs`).
- `src/ib/grid.rs` — driver `pub fn grid_tick(cfg,args)`. `mod grid;` + `pub use grid::grid_tick;` in
  `src/ib/mod.rs`. **Review-by-reading + paper acceptance** (needs a gateway).

**Edits to existing files (all additive / visibility-only — NO behavior change, prior frozen suites stay green)**
- `src/ib/trade.rs`: `fn place_with_client` → `pub(crate) fn` (visibility only). Extract
  `pub(crate) fn cancel_with_client(client:&Client, order_id:i32) -> Result<Value,AppError>` = the body of
  `cancel` from `client.cancel_order(...)` through the bounded-ack match; `cancel()` becomes
  `require_live_write_gate(cfg)?; let client=connect(cfg)?; cancel_with_client(&client, args.order_id)`.
  Byte-identical behavior; `cancel`'s public signature unchanged.
- `src/ib/account.rs` (or a small new `pub(crate)` fn there): `read_account_positions(client:&Client,
  account_filter:Option<&str>) -> Result<(AccountSnap, Vec<(String,PositionLite)>), AppError>` — ONE
  `client.account_updates(account)` drain routing `AccountValue`→`SummaryAccumulator` (reused) and
  `PortfolioValue`→`(symbol, PositionLite{qty=p.position, avg_cost=p.average_cost})`. Returns `crate::grid`
  types. `SummaryAccumulator` stays private to `account.rs`; only this typed helper is exposed.
- `src/cli.rs`: `GridTick { #[arg(long)] config: PathBuf, #[arg(long)] dry_run: bool }` command variant.
- `src/main.rs` (or wherever the command match lives): dispatch `GridTick` → `ib::grid_tick(&cfg,&args)`,
  print the JSON via the existing envelope/format path.
- `src/lib.rs`: `pub mod grid;`.
- `src/ib/mod.rs`: `mod grid;` + `pub use grid::grid_tick;`.

**Spec-paths (frozen by task)**: `tests/grid_tick.rs`. **Impl-paths**: the files above.
`spec-paths ∩ impl-paths = ∅` holds (the test file is new and distinct).

## Exact types to freeze (task pins these; impl fills bodies)

```rust
// src/grid.rs  (pure)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side { Buy, Sell }

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Cancel { order_id: i32 },
    Place  { symbol: String, side: Side, qty: f64, limit: f64 },
}

#[derive(Debug, Clone, Copy)]
pub struct AccountSnap { pub total_cash: f64, pub net_liquidation: f64 }

#[derive(Debug, Clone, Copy)]
pub struct PositionLite { pub qty: f64, pub avg_cost: f64 }

#[derive(Debug, Clone)]
pub struct OpenOrderLite { pub order_id: i32, pub symbol: String, pub side: Side, pub limit: f64, pub qty: f64 }

#[derive(Debug, Clone, serde::Deserialize)]
pub struct GridConfig {
    #[serde(default = "default_lot")]           pub lot: f64,            // 100.0
    #[serde(default = "default_cash_floor_pct")] pub cash_floor_pct: f64,// 50.0
    #[serde(rename = "symbol")]                  pub symbols: Vec<SymbolCfg>, // [[symbol]] tables
}
#[derive(Debug, Clone, serde::Deserialize)]
pub struct SymbolCfg {
    pub name: String,
    #[serde(default = "default_pct")]        pub drop_pct: f64,   // 2.0
    #[serde(default = "default_pct")]        pub rise_pct: f64,   // 2.0
    #[serde(default = "default_max_shares")] pub max_shares: f64, // 300.0
}

impl GridConfig {
    /// Read+parse toml, apply serde defaults, validate → AppError::config (exit 5) on any violation.
    pub fn load(path: &std::path::Path) -> Result<GridConfig, crate::error::AppError>;
    /// (or a separate `validate(&self) -> Result<(),String>` the loader calls)
}

/// PURE. No I/O. See ADR 0033 D-PLANNER for the full rule. round2(x) = (x*100.0).round()/100.0.
pub fn plan_grid_tick(
    cfg: &GridConfig, acct: &AccountSnap,
    positions: &std::collections::HashMap<String, PositionLite>, open: &[OpenOrderLite],
) -> Vec<Action>;
```

Notes for task/impl:
- `lot`, `qty`, `max_shares` are `f64` (IB quantities are f64 throughout `Order.total_quantity`/
  `p.position`); the ceiling test is `qty + lot <= max_shares`. Keep everything f64 to match the wire.
- `round2` on the limit only (NVDA tick $0.01; v1 assumes ≥$1 / 2-decimal names).
- Reconcile match tolerance: `(a.limit - d.limit).abs() <= 0.005 && a.qty == d.qty && a.side == d.side`.
  (`qty` equality on f64 is safe here — both are integ(lot / min(lot,qty)) values from config/positions.)
- Output ordering: push all `Cancel`s, then all `Place`s. Within a symbol, cancel the non-matching
  existing order(s) before its replacement Place.
- `plan_grid_tick` MUST NOT touch symbols absent from `cfg.symbols` (iterate `cfg.symbols`, look up
  positions/orders by name).

## Validation (GridConfig::load → AppError::config, exit 5, offline)

`symbols` non-empty; each `name` non-empty; `drop_pct`/`rise_pct` in `(0.0, 100.0)`; `lot >= 1.0`;
`cash_floor_pct` in `[0.0, 100.0]`; `max_shares >= 0.0`. `max_shares < lot` is allowed (sell-only). A toml
parse error (`toml::from_str` Err) also maps to `AppError::config`. All offline ⇒ frozen-testable with a
temp file or an inline toml string (task decides the harness; a `GridConfig::from_toml_str(&str)` inner
that `load` wraps makes the parse+validate freezable WITHOUT touching the filesystem — recommend it).

## Driver execution + JSON envelope (review-by-reading)

- Build a Buy/Sell: `build_stk_order(&symbol, Action::Buy|Sell mapped to ib Action, qty, Some(limit))`
  ⇒ LMT DAY STK (unchanged builder); then `place_with_client(&client,"grid-tick",&contract,&order,
  &account, |id,st| json!({"action":"place","side":…,"symbol":…,"qty":…,"limit":…,"order_id":id,"status":st}))`.
- Cancel: `cancel_with_client(&client, order_id)` ⇒ `{"action":"cancel","order_id":…,"status":…}`.
- Execute Cancels-first (planner already orders them); **stop on the first error**, record it, return the
  partial result (idempotent reconcile fixes up next tick; never blind-retry — ADR 0017 §4).
- `--dry-run`: run steps 1–6, then emit the planned actions with `dry_run:true` and NO order_id/status,
  return WITHOUT executing (still connected for the reads; issues zero writes).
- Envelope: `{ "account": <id>, "dry_run": <bool>, "planned": <n>, "actions": [ <per-action objects> ] }`.
  No-op tick ⇒ `"actions": []`.

## What does NOT change (grep-verifiable; review re-checks)

`build_stk_order` (4-arg body/signature), `shape_order_ack`, `place_core`, `preview_stk_option`, `place`,
`cancel`'s public signature/behavior, `require_live_write_gate`, `check_live_write_posture`,
`resolve_max_notional`, `combo_live_max_risk`, all `option-*` verbs, `src/config.rs`. Raw `place_order`/
`cancel_order` appear ONLY in `trade.rs` (ADR 0017 grep invariant). Every existing frozen suite stays green
and byte-identical.

## For task (next stage)

1. Freeze `tests/grid_tick.rs` per ADR 0033 "Freeze coverage": the seven `plan_grid_tick` cases + the two
   `GridConfig` parse cases (valid→defaults, malformed/negative-%→config). Import from
   `oh_my_ib::grid::{plan_grid_tick, GridConfig, Action, Side, AccountSnap, PositionLite, OpenOrderLite}`.
   Recommend a `GridConfig::from_toml_str` seam so parse+validate freezes with no filesystem.
2. One card is natural (the feature is cohesive: pure module + driver + wiring), but if split, keep the
   pure planner+config as card 01 (the frozen heart) and the driver+wiring as card 02 (review-by-reading);
   both under ONE `spec-rev`. Record `full-verify` = `[cargo build, cargo test, cargo clippy --all-targets
   -- -D warnings]` (already in current.json).
3. The clippy-on-stub trap (recurring): the frozen test imports symbols that don't exist yet ⇒ won't
   compile-clean. Pin the RED via the unresolved import (compile-fail), exactly as outside-rth did — do NOT
   land a stub in `src/` from the task stage (that's impl-path).
4. Impl handoff → omp (goal-driven-impl-claude); review → codex (check): freeze gate + full-suite green +
   paper acceptance (criterion 8). Model for task/review: frontier SOTA; impl: capable-local OK.
