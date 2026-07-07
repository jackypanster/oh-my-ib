# CONTEXT — sma-tick (domain language)

- **sma-tick** — `omi sma-tick [SYMBOL=QQQ] --lot 10 [--sma 200] [--dry-run]`: the ACTIVE 200-day
  month-end timing executor. Each run reconciles the position to the signal's binary target and places
  one order. Paper-only v1. The write counterpart to the read-only `sma-signal`.
- **binary target** — HOLD ⇒ hold exactly `lot` shares; EXIT ⇒ hold 0. All-in / all-out at a fixed size
  (the strategy strategy-lab validated). NOT a ladder (never "buy `lot` more each month").
- **reconcile tick** — `delta = target − current_qty`; `Buy(delta)` / `Sell(−delta)` / `Noop`. Works from
  any current qty. The pure `plan_sma_tick(state, current_qty, lot) -> TickAction` — the frozen heart.
- **marketable LMT** — the order type: Buy at `round2(latest_close × 1.02)`, Sell at `× 0.98`. Fills at
  the next open like a MKT but rests cleanly (`PreSubmitted`) when the market is closed — avoids the MKT
  `[399] queued-to-open` error and reuses `place_with_client` unchanged. `latest_close` from the SmaSignal.
- **paper-only (v1)** — `sma_tick_cmd` refuses `cfg.port == LIVE_PORT` (config, offline). 10 QQQ ≈ $7.2k
  ≫ the $500 notional cap → live would refuse it; live is a separate future decision.
- **the gateway** — `sma_tick_cmd`: refuse-live → connect → `signal_for` (reuse sma-signal fetch+fn) →
  `positions()` current qty → `plan_sma_tick` → execute via `build_stk_order` + `place_with_client` →
  JSON. Review-by-reading; not frozen.
- **containment** — no raw `place_order` in sma_tick.rs; composes trade.rs choke points (ADR 0017), like
  grid-tick. `signal_for` = a `pub(crate)` fetch helper extracted from `sma_signal_cmd` (shared, so
  sma-signal stays byte-identical).
- **--dry-run** — show signal + current/target/action, place nothing.
- **not this feature** — monthly scheduling + Telegram confirmation (ops glue, extend `sma-monthly`); the
  live path; multi-symbol; MKT.

Unchanged domain (do not touch): `sma_signal` frozen behavior, grid-tick, `trade.rs` write path + the
gates + notional cap, `positions`/`history`.
