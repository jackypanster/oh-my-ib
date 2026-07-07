# CONTEXT — sma-signal (domain language)

- **sma-signal** — `omi sma-signal [SYMBOL...] [--sma 200]`: a READ-ONLY command that reports, per symbol,
  the Faber 200-day timing signal. No orders, no gate. `omi sma-signal` (no args) signals current holdings.
- **200-day SMA** — the simple moving average of the last 200 daily CLOSES. The trend line: price above it
  = up-trend (hold); below = broken (step aside). Validated by `strategy-lab` as the one edge over hold.
- **Faber month-end signal / standing signal** — the rule is evaluated only at the **last completed
  month-end** (last trading day of the previous calendar month), not every day. That standing decision
  (HOLD or EXIT) holds all month; monthly cadence is the whipsaw filter. `state`: `HOLD` (month-end close ≥
  SMA), `EXIT` (below), `INSUFFICIENT` (< n bars of history).
- **month-end** — the last TRADING day of a calendar month present in the bars (group by (year,month), take
  the group's last bar), NOT the calendar 30th/31st.
- **as-of** — the (year,month) whose month-end the standing signal was evaluated at = the last month
  STRICTLY BEFORE the in-progress final month (so the signal doesn't flicker mid-month).
- **drift context (latest_*)** — the current (final bar) close vs the current 200-day SMA + distance. Tells
  you whether a flip is likely at the next month-end. Informational; the `state` is the standing decision.
- **distance_pct** — `(close − sma) / sma × 100`. How far above/below the line, in %.
- **the pure seam** — `sma_signal(bars: &[Bar], n) -> SmaSignal`, `Bar { ym:(i32,u32), close:f64 }`. No
  ibapi, no I/O ⇒ the whole frozen surface (`tests/sma_signal.rs`).
- **the gateway** — `sma_signal_cmd(cfg, args)`: resolve symbols → fetch 2Y Day/Trades bars
  (`historical_data`, reused from `history`) → strip `BarTimestamp` (`Date`/`DateTime`) to `(year,month)` →
  run the pure fn → JSON. Review-by-reading; not frozen.
- **read-only posture** — no `--live`/`OMI_ALLOW_LIVE`/`place_order`/`cancel_order`; default paper port
  (market data identical). ADR 0017 write-containment does NOT apply. Delayed data is fine (historical
  closes, not live quotes).
- **Phase 2 (deferred) — `sma-tick`** — an ACTIVE cron command that auto-enters/exits to the signal (write
  path, paper-first, gated). NOT this feature.

Unchanged domain (do not touch): `history`/`quote`/`positions` behavior, `trade.rs`, the live gate, the
notional cap. sma-signal only ADDS a new read module + CLI verb + a direct `time` dep.
