# PRD — sma-signal

Stage: prd · feature: sma-signal · repo: jackypanster/oh-my-ib · branch: main
Author: cc. Provenance: the `strategy-lab` bake-off (2026-07-07) found the cost-anchored grid
retires and **200-day SMA month-end timing** is the one systematic edge worth surfacing (keeps
~buy-hold return, cuts mean max-drawdown -50.7%→-36.4%, MAR-beats-hold 5/8 liquid names). Two
operator decisions locked via /think AskUserQuestion: **read-only signal command** (no write
path) + **month-end cadence** (Faber, least whipsaw).

## Problem

The operator will hold liquid US names (QQQ/NVDA/MU/…) and wants a mechanical answer to "am I
still supposed to be holding this, or has it broken down?" — instead of an emotional gut call.
The 200-day SMA month-end rule (Meb Faber) is that mechanical rule, and it back-tests as the only
edge over buy-and-hold on this panel. `omi` has the data (`history` = reqHistoricalData) and the
read-command scaffolding, but nothing computes the signal.

## Goal

Add `omi sma-signal [SYMBOL...]`: a **read-only** command that, per symbol, reports the standing
**Faber 200-day timing signal** — at the last completed month-end, is the close above its 200-day
SMA (**HOLD**) or below (**EXIT**, step to cash) — plus the current (latest-bar) drift toward the
next month-end flip. No orders, no gate. You act manually. (Active auto-trading `sma-tick` is an
explicit Phase 2, out of scope here.)

## Code reality (verified)

- **Bars are already fetchable**: `history.rs:17` — `client.historical_data(&contract, BarSize::Day)
  .what_to_show(WhatToShow::Trades).duration("2 Y").fetch()` → `data.bars` (each `b.date`, `b.close`,
  …). 2 years ≈ 500 daily bars ⇒ enough for a 200-day SMA + ~24 month-ends. `sma-signal` reuses this
  exact call. **Verified live**: `omi history NVDA --bar 1d --duration 1Y` returns 251 clean bars.
- **Delayed data is irrelevant** — the signal is computed from historical daily CLOSES, not a live
  quote, so the account's delayed real-time feed does not affect it. (Contrast grid-tick, which also
  didn't care; here it's even cleaner — pure historical.)
- **Read-command pattern**: read verbs (`quote`/`history`/`positions`) live in their own `src/ib/*.rs`
  modules, connect, shape JSON, no gate, default paper port (market data is identical across ports).
  `sma-signal` follows this — it is NOT a write command, so ADR 0017 containment (trade.rs) does not
  apply; no `--live`/`OMI_ALLOW_LIVE` gating.
- **No-args ergonomic**: `positions.rs:positions()` returns held symbols → `sma-signal` with no
  positional args can signal the current holdings.

## Decisions (provenance-tagged; ✅ = human-confirmed /think AskUserQuestion)

- **D-READONLY — read-only signal command, no write path.** ✅. Reports HOLD/EXIT; the operator
  executes manually. No orders, no live gate, default paper port (market data same). New READ module,
  NOT `trade.rs`. Smallest, fastest, lowest-risk; fits oh-my-ib's thin-wrapper character.
- **D-MONTHEND — the signal is evaluated at the last COMPLETED month-end (Faber cadence).** ✅.
  Not the daily close — month-end evaluation is what minimizes whipsaw (the 2022-H1 false signals in
  the backtest are mostly filtered out). The "standing signal" is whatever was decided at the most
  recent completed month-end; it does not change intra-month. The command ALSO reports the latest-bar
  close vs current SMA as **drift context** (are we about to flip at next month-end?).
- **D-PURE-SEAM — a pure `sma_signal(bars, n)` is the frozen heart; the gateway fetch is thin.** ✅
  (design, mirrors grid-tick). The month-end + SMA + HOLD/EXIT logic is pure and offline-testable
  (given a bar series → a deterministic signal). The gateway wrapper (fetch 2Y bars, resolve symbols)
  is review-by-reading. Frozen surface = `sma_signal`; tests in `tests/sma_signal.rs`.
- **D-SYMBOLS — 1+ positional symbols; no args ⇒ signal current positions.** ✅ (ergonomic). `omi
  sma-signal NVDA MU QQQ` signals those; `omi sma-signal` (no args) reads `positions()` and signals
  each held name. The symbol-resolution is gateway (review-by-reading), not frozen.
- **D-DATA — reuse `historical_data` (Day/Trades/"2 Y").** ✅. 2Y buffer ⇒ 200-day SMA + month-end
  detection with margin. `--sma <n>` overrides the window (default 200); `--duration` not exposed
  (fixed 2Y internally — enough for any n≤~450). Insufficient history (fewer than n bars, e.g. a
  recent IPO) ⇒ a graceful `state: "INSUFFICIENT"` per-symbol, not a crash.

## The pure signal contract (this is what `pipeline-task` freezes)

```
Bar        { ym: (i32, u32), close: f64 }        // (year, month) extracted from the IB bar date + close
SmaSignal  {
  as_of_month_end: (i32, u32),   // the last COMPLETED month (year, month) the signal was evaluated at
  month_end_close: f64,          // close of that month-end's last trading day
  sma: f64,                      // n-day SMA over closes up to & incl. that month-end
  state: "HOLD" | "EXIT" | "INSUFFICIENT",
  distance_pct: f64,             // (month_end_close - sma) / sma * 100
  latest_close: f64,             // most recent bar close (drift context)
  latest_sma: f64,               // n-day SMA as of the latest bar
  latest_distance_pct: f64,
  bars_used: usize,
}

sma_signal(bars: &[Bar], n: usize) -> SmaSignal          // bars ascending by date

rule:
  if bars.len() < n            → state = INSUFFICIENT (other numeric fields best-effort / 0)
  month_ends = last bar of each distinct (year,month) group
  as_of = the latest month_end whose (year,month) is STRICTLY BEFORE the (year,month) of the last bar
          (exclude the in-progress current month); if only one month present → that month's end
  sma@as_of = mean(close of the n bars ending at the as_of index)   (needs ≥ n bars up to as_of, else INSUFFICIENT)
  state = HOLD if month_end_close >= sma@as_of else EXIT
  latest_* = same math at the final bar index (current drift)
```

Deterministic, offline, no I/O — the whole test surface. (Month-end = last TRADING day in the bar
series for that calendar month, NOT the calendar last day.)

## Scope

- **IN** `src/ib/signal.rs` (NEW read module): the pure `sma_signal` + the gateway `sma_signal_cmd(cfg,
  args)` (resolve symbols → for each: fetch 2Y Day/Trades bars, map to `Bar{ym,close}`, run
  `sma_signal`, collect) → JSON `{ signals: [ {symbol, ...SmaSignal} ] }`.
- **IN** `src/cli.rs`: `SmaSignal(SmaSignalArgs)` variant; `struct SmaSignalArgs { symbols: Vec<String>,
  #[arg(long, default_value = "200")] sma: usize }`.
- **IN** `src/main.rs` (dispatch) + `src/ib/mod.rs` (`mod signal; pub use signal::{sma_signal, sma_signal_cmd};`).
- **IN** NEW frozen spec `tests/sma_signal.rs`.
- **OUT** (non-scope): any order placement / auto-trading (that is Phase 2 `sma-tick`); the live gate /
  write path (read-only, none needed); other indicators/MAs (RSI, crossover — deferred); a CLI backtester
  (strategy-lab owns backtesting); `--duration` flag (fixed 2Y internally); modifying `positions`/`history`.

## Success criteria (acceptance)

1. **Pure `sma_signal` (offline, FROZEN):** a bar series whose last completed month-end close is ABOVE
   the n-SMA ⇒ `state == HOLD`; BELOW ⇒ `EXIT`; `distance_pct` = (close−sma)/sma·100. [frozen]
2. **Month-end selection (FROZEN):** `as_of` is the last COMPLETED month (the in-progress final month is
   excluded); the SMA is computed as of that month-end, not the latest bar. [frozen]
3. **Insufficient history (FROZEN):** `< n` bars ⇒ `state == "INSUFFICIENT"` (no panic). [frozen]
4. **Drift context (FROZEN):** `latest_*` fields reflect the final bar's close vs current n-SMA. [frozen]
5. **CLI (operator, paper `:4002`):** `omi sma-signal NVDA MU QQQ` returns a signal per symbol with
   HOLD/EXIT + distances; `omi sma-signal` (no args) signals current positions. Read-only, no gate, works
   with the delayed-data account. [operator]
6. `cargo build` · full `cargo test` · `cargo clippy --all-targets -- -D warnings` green; all prior
   suites byte-identical. [verify]

## Gotchas (project-specific traps the next nodes MUST know)

- **Month-end = last TRADING day of the month in the bar series**, not the calendar 30th/31st. Detect by
  grouping bars on (year, month) and taking each group's last bar.
- **Use the last COMPLETED month-end** — exclude the current in-progress month, else the "signal" flickers
  intra-month. That standing signal is the point of the month-end cadence (D-MONTHEND).
- **SMA is computed AS OF the month-end** (Faber), not just the latest 200-day average. Report BOTH
  (month-end signal = the decision; latest = drift context).
- **Read-only — do NOT add `--live`/`OMI_ALLOW_LIVE` or any gate.** Market data is identical across
  ports; this is a read like `quote`/`history`. Containment (ADR 0017) is irrelevant (no writes).
- **The ibapi bar `date` type** (`b.date`) must yield (year, month) for grouping — arch confirms the exact
  accessor (it is currently only `{:?}`-formatted in `history.rs`; the pure fn takes already-extracted ym).
- **Delayed data is fine** — historical closes, not live quotes.
- Insufficient-history names (new IPOs < n bars) ⇒ `INSUFFICIENT`, never a crash.

## Verify

`cargo build` · `cargo test` (new `tests/sma_signal.rs` red→green; all prior suites green) · `cargo clippy
--all-targets -- -D warnings`. Operator paper: criterion 5 on `:4002` (needs a gateway for the bar fetch).
No gateway needed for the frozen offline `sma_signal` tests.

## For arch (next stage)

1. Module placement: new READ module `src/ib/signal.rs` (pure `sma_signal` + gateway `sma_signal_cmd`);
   confirm it is NOT under trade.rs and needs no gate. Author a short ADR (0034) recording the read-only
   200-SMA month-end signal + provenance (strategy-lab) + the Phase-2 `sma-tick` deferral.
2. Confirm the exact ibapi bar `date` accessor for (year, month) extraction (the one real unknown), and
   how the gateway maps `data.bars` → `Bar{ym, close}`. Factor a small shared bar-fetch helper or reuse
   `historical_data` inline.
3. Pin the `Bar` / `SmaSignal` types + `sma_signal` signature precisely for the task freeze.
4. Confirm the no-args→`positions()` symbol resolution + the JSON output shape; CONTEXT.md glossary
   ("Faber month-end signal", "standing signal", "drift").
