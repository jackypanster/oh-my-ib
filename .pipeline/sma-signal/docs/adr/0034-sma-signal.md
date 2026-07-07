# ADR 0034 — sma-signal (read-only Faber 200-day month-end timing signal)

Status: accepted · 2026-07-07 · feature: sma-signal · read-only (ADR 0017 write-containment N/A);
reuses `history` (reqHistoricalData). Provenance: the `strategy-lab` bake-off.

## Context

The `strategy-lab` backtest (2026-07-07, 4 strategies × 8 liquid US names × 2015-2026) found the
cost-anchored grid retires (it sits ~99% in cash on trending names) and the **only** systematic edge over
buy-and-hold is **200-day SMA month-end timing** (Meb Faber): it keeps ~the same return while cutting mean
max-drawdown -50.7%→-36.4% and beats buy-hold on risk-adjusted MAR in 5/8 names. The operator wants a
mechanical "am I still supposed to hold this?" answer instead of an emotional call. `omi` already fetches
daily bars (`history` = reqHistoricalData) but computes no signal.

## Decision — a read-only `omi sma-signal [SYMBOL...]` command

Per symbol, report the standing Faber signal: at the **last completed month-end**, is the close **≥** its
n-day SMA (**HOLD**) or **<** it (**EXIT** → step to cash), plus the current-bar drift toward the next
month-end flip. Read-only — no orders, no gate. The operator executes manually. A pure `sma_signal(bars,n)`
seam is the frozen heart; a thin gateway fetches bars and strips the ibapi date type.

### D-READONLY — read command, no write path, no gate

Reports HOLD/EXIT; no order placement. No `--live`/`OMI_ALLOW_LIVE`, default paper port (historical market
data is identical across ports). Lives in a new READ module `src/ib/signal.rs`, NOT `trade.rs` — ADR 0017
containment is about WRITE calls and does not apply. Smallest, fastest, lowest-risk surface; fits the
thin-wrapper character. Active auto-trading (`sma-tick`) is an explicit, separate Phase 2 (deferred here).

### D-MONTHEND — signal evaluated at the last COMPLETED month-end (Faber cadence)

Not the daily close. Month-end evaluation is what minimizes whipsaw — in the backtest the 2022-H1 flurry of
daily false signals mostly vanishes at monthly cadence. The "standing signal" is decided at the most recent
completed month-end and does not change intra-month. The command ALSO reports the latest-bar close vs the
current SMA as **drift context** (is a flip likely at the next month-end?). Month-end = the last TRADING day
of a calendar month present in the bar series (group bars by (year,month), take each group's last bar), and
the SMA is computed AS OF that month-end (not merely the latest 200-day average).

### D-PURE-SEAM — pure `sma_signal(bars, n)` is the frozen surface; the gateway is thin

The month-end + SMA + HOLD/EXIT/INSUFFICIENT logic is pure (`&[Bar{ym,close}] -> SmaSignal`), offline,
deterministic ⇒ the entire freeze surface (`tests/sma_signal.rs`). The gateway `sma_signal_cmd` does I/O
only: resolve symbols (args, or `positions()` when none), fetch 2Y Day/Trades bars via the existing
`historical_data` call, and strip `BarTimestamp` → `(year,month)` before handing to the pure fn. The bar
`date` is `ibapi::…::BarTimestamp` = `Date(time::Date)` | `DateTime(OffsetDateTime)`; `(year, month)` come
from `time`'s `.year()`/`.month()` accessors. Keeping ibapi types out of the pure fn is what makes it
freezable.

### D-DATA — reuse `historical_data`, fixed 2-year Day window

`historical_data(stock, Day).what_to_show(Trades).duration("2 Y").fetch()` ≈ 500 bars ⇒ a 200-day SMA plus
~24 month-ends with margin. `--sma <n>` overrides the window (default 200); `--duration` is not exposed
(fixed 2Y internally covers any n ≤ ~450). Fewer than `n` bars (recent IPO) ⇒ `state: INSUFFICIENT`, never
a panic. Delayed market data is irrelevant — the signal is historical daily closes.

## Consequences

- `omi sma-signal NVDA MU QQQ` → a HOLD/EXIT signal per name + distances; `omi sma-signal` (no args) →
  signals the current positions. Read-only, works on the delayed-data paper account, no gate.
- New crate surface: `pub mod`-level `signal` with `sma_signal`/`sma_signal_cmd`/`Bar`/`SmaSignal`/
  `SignalState`; a direct `time` dep (already transitive via ibapi). No existing behavior changes.
- The operator now has the mechanical exit rule the backtest validated — decoupling "when to step aside"
  from gut feel. Executing on it stays manual (Phase 1); Phase 2 `sma-tick` could automate entry/exit.
- Not a promise: the signal surfaces a historically-edged rule; forward performance is not guaranteed
  (documented, not asserted).

## Freeze coverage

- **FROZEN** (`tests/sma_signal.rs`, offline): `sma_signal` — month-end-close ≥ SMA ⇒ Hold, < ⇒ Exit;
  `distance_pct`; last-COMPLETED-month-end selection (in-progress final month excluded; SMA as of the
  month-end, not the latest bar); `< n` bars ⇒ Insufficient (no panic); `latest_*` drift fields.
- **REVIEW-BY-READING**: the gateway `sma_signal_cmd` (symbol resolution incl. no-args→positions, the
  `historical_data` fetch, the `BarTimestamp → (y,m)` strip, JSON shape), the read-only posture (no gate,
  no write symbols — grep), the CLI wiring; prior suites byte-identical.
- **OPERATOR ACCEPTANCE** (paper `:4002`): `omi sma-signal NVDA MU QQQ` returns sensible HOLD/EXIT vs each
  name's 200-day line; `omi sma-signal` signals held positions.

## Alternatives rejected

- **Active auto-trading `sma-tick` now.** Write path + gating + position-sizing decisions — larger surface;
  the read-only signal delivers the decision value first. Deferred to Phase 2 (operator choice).
- **Daily cadence.** More whipsaw (2022-H1 false signals); month-end is Faber's whipsaw control (D-MONTHEND).
- **Put the logic in `trade.rs` / add a gate.** It performs no writes; forcing it through the write module
  or a live gate is wrong (D-READONLY).
- **Live real-time quote for the signal.** Unnecessary and blocked by delayed data; historical closes are
  the correct, available input (D-DATA).
