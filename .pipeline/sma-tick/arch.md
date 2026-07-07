# arch — sma-tick

Stage: arch · feature: sma-tick · author: cc (grill-with-docs). Binding decisions in ADR 0035.
Pins the module boundary + exact types for the task freeze. The 4 locked decisions (binary target;
QQQ+lot10+flags; paper-only; pure planner) are NOT re-opened.

## Chosen shape — pure `plan_sma_tick` (frozen) + thin gateway (review-by-reading), WRITE-orchestration

```
   omi sma-tick [SYMBOL=QQQ] --lot 10 [--sma 200] [--dry-run]
        │  main.rs → crate::ib::sma_tick_cmd(&cfg, &args)
        ▼
   ┌──  src/ib/sma_tick.rs  (WRITE-orchestration; paper-only; composes trade.rs choke points)  ──┐
   │ sma_tick_cmd(cfg, args):  (gateway, review-by-reading)                                       │
   │   if cfg.port == LIVE_PORT → AppError::config("sma-tick is paper-only in v1", "sma-tick")   │
   │   client = ib::connect(cfg)?;  account = ib::resolve_account(&client, cfg)?                  │
   │   (sig, ) = signal::signal_for(&client, &symbol, args.sma)?   ── reuse sma-signal fetch+fn   │
   │   current_qty = qty of `symbol` in positions(cfg)  (0 if absent)                             │
   │   action = crate::ib::plan_sma_tick(sig.state, current_qty, args.lot)    ── PURE, FROZEN      │
   │   if args.dry_run → return shape(sig, current, target, action, dry_run=true)                 │
   │   else match action:                                                                         │
   │     Buy{qty}  → price = round2(sig.latest_close * 1.02)  (marketable LMT)                     │
   │     Sell{qty} → price = round2(sig.latest_close * 0.98)                                       │
   │     → (contract, order) = build_stk_order(symbol, side, qty, Some(price))                     │
   │     → place_with_client(&client, "sma-tick", &contract, &order, &account, ack)               │
   │     Noop → no order                                                                          │
   │   → json!({symbol, signal, as_of, current_qty, target_qty, action, order?})                  │
   └───────────────────────────────────────────────────────────────────────────────────────────────┘
        │ composes (NO raw place_order — ADR 0017)
        ▼  build_stk_order + place_with_client (trade.rs) · sma_signal (signal.rs) · positions()
   ┌──  the PURE seam (FROZEN — tests/sma_tick.rs)  ──────────────────────────────────────────────┐
   │ plan_sma_tick(state: SignalState, current_qty: f64, lot: f64) -> TickAction                   │
   └───────────────────────────────────────────────────────────────────────────────────────────────┘
```

## D-ORDER resolved — marketable LMT (NOT MKT)

Live evidence (this session): a **LMT** at a price returns a clean `PreSubmitted` 6-key ack = success
(the NVDA @188 test); a **MKT while the market is closed** comes back as `[399] … will not be placed
until 09:30` which the place path surfaces as `AppError::data` (an ERROR). So MKT would make sma-tick
fail every time it runs outside RTH. **Use a marketable LMT** instead: `Buy` at `round2(latest_close *
1.02)`, `Sell` at `round2(latest_close * 0.98)` — aggressive enough to fill at the next open (like a
MKT), but it rests cleanly as `PreSubmitted` when the market is closed (no [399] error) and **reuses
`place_with_client` UNCHANGED**. It is also live-shaped (live requires LMT, ADR 0030) for a future
promotion. `latest_close` comes from the `SmaSignal` already computed — no extra fetch. (Gap risk beyond
2% is rare and self-heals next tick; the buffer is a fixed 2% in v1.)

## Exact types to freeze (task pins these)

```rust
// src/ib/sma_tick.rs  (pure part — FROZEN)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TickAction { Buy { qty: f64 }, Sell { qty: f64 }, Noop }

/// PURE. state = the Faber signal; reconcile current_qty toward the binary target (Hold ⇒ lot, Exit ⇒ 0,
/// Insufficient ⇒ no trade). See ADR 0035.
pub fn plan_sma_tick(state: SignalState, current_qty: f64, lot: f64) -> TickAction;
```

**Algorithm:**
```
target = match state { Hold => lot, Exit => 0.0, Insufficient => return Noop }
delta  = target - current_qty
if delta >  1e-9 → Buy { qty: delta }
else if delta < -1e-9 → Sell { qty: -delta }
else → Noop
```
`SignalState` is `crate::ib::SignalState` (PR #31). f64 throughout (matches qty/positions). No `==` on
floats (clippy float_cmp) — the fn uses `>`/`<` only; the frozen test uses an `approx` helper.

## Component boundaries + write-set

**New**: `src/ib/sma_tick.rs` (pure `plan_sma_tick` + `TickAction` + gateway `sma_tick_cmd` + JSON shape).
`spec-paths` = `tests/sma_tick.rs`.

**Edits (additive)**:
- `src/ib/signal.rs`: extract `pub(crate) fn signal_for(client: &Client, symbol: &str, n: usize) ->
  Result<SmaSignal, AppError>` (the fetch-2Y-bars + map + `sma_signal` currently inline in
  `sma_signal_cmd`); `sma_signal_cmd` calls it too (behavior byte-identical → its frozen tests + the JSON
  stay green). sma_tick_cmd reuses it.
- `src/ib/mod.rs`: `mod sma_tick;` + `pub use sma_tick::{plan_sma_tick, sma_tick_cmd, TickAction};`
- `src/cli.rs`: `SmaTick(SmaTickArgs)` + `struct SmaTickArgs { symbol: Option<String>,
  #[arg(long, default_value_t = 10.0)] lot: f64, #[arg(long, default_value_t = 200)] sma: usize,
  #[arg(long)] dry_run: bool }` (symbol default "QQQ" resolved in the gateway).
- `src/main.rs`: dispatch `SmaTick(a) => ib::sma_tick_cmd(&cfg, a)`.

**Impl-paths**: `src/ib/sma_tick.rs`, `src/ib/signal.rs`, `src/ib/mod.rs`, `src/cli.rs`, `src/main.rs`.
`spec-paths ∩ impl-paths = ∅`.

## Read/write posture (grep-verifiable)

Paper-only: hard-refuse `cfg.port == LIVE_PORT` before connect. NO raw `place_order`/`cancel_order` in
`sma_tick.rs` — it composes `build_stk_order` + `place_with_client` (ADR 0017 containment holds; sma-tick
is a sanctioned choke-point consumer, like grid-tick/place_core). The double live gate / notional cap are
untouched (never reached — paper-only).

## For task (next stage)

1. Freeze `tests/sma_tick.rs` (spec-paths), importing `oh_my_ib::ib::{plan_sma_tick, TickAction,
   SignalState}`. Cover: Hold+0+lot10 ⇒ Buy 10; Hold+10 ⇒ Noop; Hold+4 ⇒ Buy 6; Hold+15 ⇒ Sell 5;
   Exit+10 ⇒ Sell 10; Exit+0 ⇒ Noop; Insufficient ⇒ Noop. `approx` helper (clippy float_cmp). RED via the
   unresolved `oh_my_ib::ib::plan_sma_tick` import — no src/ stub.
2. One card (frozen pure planner; gateway + `signal_for` extraction + wiring = review-by-reading).
   verify = `[cargo build, cargo test --test sma_tick]`.
3. Impl → omp; review → codex (freeze gate + full-suite + paper-only grep + containment grep + paper
   acceptance: `omi sma-tick QQQ --dry-run` on :4002).
