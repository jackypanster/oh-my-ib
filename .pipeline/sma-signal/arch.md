# arch — sma-signal

Stage: arch · feature: sma-signal · author: cc (grill-with-docs). Binding decisions in ADR 0034.
This pins the module boundary + the exact Rust types so `pipeline-task` can freeze without re-deciding.
The 2 locked decisions (read-only; month-end cadence) are NOT re-opened.

## Chosen shape — pure `sma_signal` (frozen) + thin gateway (review-by-reading), READ-only

```
   omi sma-signal [SYMBOL...] [--sma 200]
            │  main.rs dispatch → crate::ib::sma_signal_cmd(&cfg, &args)
            ▼
   ┌───────  src/ib/signal.rs  (NEW read module — NO gate, NOT trade.rs)  ────────────┐
   │ sma_signal_cmd(cfg, args):  (gateway, review-by-reading)                          │
   │   syms = args.symbols  OR  (empty ⇒ held symbols from positions(cfg))             │
   │   client = ib::connect(cfg)?                                                      │
   │   for sym in syms:                                                                │
   │     data = client.historical_data(Contract::stock(sym), BarSize::Day)            │
   │              .what_to_show(Trades).duration("2 Y".to_duration()).fetch()?         │
   │     bars = data.bars.map(|b| Bar { ym: ym_of(&b.date), close: b.close })          │
   │     sig  = crate::ib::sma_signal(&bars, args.sma)      ── PURE, FROZEN             │
   │   → json!({ "signals": [ { "symbol": sym, ...shape(sig) } ] })                    │
   │                                                                                   │
   │ ym_of(&BarTimestamp) -> (i32,u32):   (gateway helper — maps ibapi date → (y,m))   │
   │   BarTimestamp::Date(d)      => (d.year(), u8::from(d.month()) as u32)             │
   │   BarTimestamp::DateTime(dt) => (dt.year(), u8::from(dt.month()) as u32)           │
   └───────────────────────────────────────────────────────────────────────────────────┘
            │ calls (pure, no ibapi, no client)
            ▼
   ┌───────  the PURE seam (FROZEN, offline — tests/sma_signal.rs)  ────────────────────┐
   │ sma_signal(bars: &[Bar], n: usize) -> SmaSignal                                    │
   │ Bar { ym: (i32,u32), close: f64 }   (ascending by date; ibapi type already stripped)│
   └───────────────────────────────────────────────────────────────────────────────────┘
```

No cycle. The pure seam has zero ibapi/gateway deps → 100% offline-testable (the whole freeze surface).
The gateway does I/O + the `BarTimestamp → (y,m)` strip only.

## The ibapi bar date (the one resolved unknown)

`ibapi::market_data::historical::Bar.date: BarTimestamp` (crate 3.1, `historical/mod.rs:164`). It is an
enum: `Date(time::Date)` for daily/weekly/monthly (our case — renders `Date(2026-06-08)`) and
`DateTime(time::OffsetDateTime)` for intraday. `time::Date`/`OffsetDateTime` expose `.year() -> i32` and
`.month() -> time::Month`; `u8::from(month)` gives 1..=12. `time` is already a transitive dep via ibapi —
arch adds it as a **direct** `Cargo.toml` dep (version-matched) so `signal.rs` can name `BarTimestamp`
(re-exported by ibapi) and call the accessors cleanly. Only the gateway touches `time`/`BarTimestamp`; the
pure fn sees only `(i32,u32)`.

## Exact types to freeze (task pins these; impl fills bodies)

```rust
// src/ib/signal.rs  (pure part — FROZEN)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalState { Hold, Exit, Insufficient }   // JSON: "HOLD" | "EXIT" | "INSUFFICIENT"

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bar { pub ym: (i32, u32), pub close: f64 }   // (year, month), ascending by date

#[derive(Debug, Clone, PartialEq)]
pub struct SmaSignal {
    pub state: SignalState,
    pub as_of_month_end: (i32, u32),   // last COMPLETED (year, month) the signal was evaluated at
    pub month_end_close: f64,
    pub sma: f64,                      // n-SMA over the n closes ending at that month-end
    pub distance_pct: f64,             // (month_end_close - sma) / sma * 100
    pub latest_close: f64,             // final bar close (drift context)
    pub latest_sma: f64,               // n-SMA at the final bar
    pub latest_distance_pct: f64,
    pub bars_used: usize,              // bars.len()
}

/// PURE. No I/O, no ibapi. See ADR 0034 for the rule.
pub fn sma_signal(bars: &[Bar], n: usize) -> SmaSignal;
```

**Algorithm (ADR 0034 D-RULE):**
```
bars_used = bars.len()
if bars.len() < n  → SmaSignal { state: Insufficient, as_of_month_end: last bar ym (or (0,0) if empty),
                                 all f64 fields 0.0, bars_used }            // no panic, no slicing < n
// month-end index of each distinct (year,month) run = the LAST index of that run (bars ascending)
last_ym = bars.last().ym
// as_of index = last index whose ym is STRICTLY BEFORE last_ym (exclude the in-progress final month);
//   if every bar shares last_ym (only one month present) → use the final index
as_of_idx = max i such that bars[i].ym < last_ym   (else bars.len()-1)
sma@idx(i) = mean(close of bars[i-n+1 ..= i])       // requires i+1 >= n, else Insufficient
if as_of_idx + 1 < n → Insufficient
state = Hold if bars[as_of_idx].close >= sma@as_of_idx else Exit
latest_* computed at idx = bars.len()-1 (drift; if len < n those were already Insufficient)
distance_pct = (close - sma)/sma*100   (sma>0 guaranteed for real price data)
```

Notes for task/impl:
- `ym` comparison is plain tuple `<` ((year,month) lexicographic = chronological). No dates in the pure fn.
- `close` is f64 → the frozen test must avoid clippy `float_cmp` (use an approx/epsilon helper, as
  `tests/grid_tick.rs` did — mirror it).
- `Insufficient` fields: set numeric to 0.0 and `as_of_month_end` to the last bar's ym (or (0,0) if empty)
  — deterministic, testable, never slices out of range.

## Component boundaries + write-set (for the task card)

**New**: `src/ib/signal.rs` (pure `sma_signal` + `SignalState`/`Bar`/`SmaSignal` + gateway
`sma_signal_cmd` + `ym_of` helper + a `shape(sig)->Value`). `spec-paths` = `tests/sma_signal.rs`.

**Edits (additive)**:
- `src/ib/mod.rs`: `mod signal;` + `pub use signal::{sma_signal, sma_signal_cmd, Bar, SmaSignal, SignalState};`
- `src/cli.rs`: `SmaSignal(SmaSignalArgs)` + `struct SmaSignalArgs { symbols: Vec<String>,
  #[arg(long, default_value_t = 200)] sma: usize }` (positional `symbols`, optional `--sma`).
- `src/main.rs`: dispatch `SmaSignal(a) => ib::sma_signal_cmd(&cfg, a)` through the existing JSON/format path.
- `Cargo.toml`: add `time` as a direct dep (version-matched to ibapi's).

**Impl-paths**: `src/ib/signal.rs`, `src/ib/mod.rs`, `src/cli.rs`, `src/main.rs`, `Cargo.toml`.
`spec-paths ∩ impl-paths = ∅` (tests/sma_signal.rs is new, distinct).

## Read-only posture (grep-verifiable; review re-checks)

NO `require_live_write_gate` / `OMI_ALLOW_LIVE` / `place_order` / `cancel_order` anywhere in `signal.rs`.
Default paper port; market data is identical across ports. ADR 0017 (write containment) does not apply —
this is a read like `quote`/`history`. No existing file's behavior changes (all additive).

## For task (next stage)

1. Freeze `tests/sma_signal.rs` (spec-paths), importing `oh_my_ib::ib::{sma_signal, Bar, SmaSignal,
   SignalState}`. Cover ADR 0034 Freeze coverage: month-end-above ⇒ Hold; below ⇒ Exit; distance_pct;
   last-completed-month-end selection (excludes the in-progress final month; SMA as of month-end, not
   latest); `< n` bars ⇒ Insufficient (no panic); latest_* drift fields. Use an `approx` helper (clippy
   float_cmp). RED via the unresolved `oh_my_ib::ib::sma_signal` import — NO src/ stub.
2. One card (cohesive read feature): frozen pure `sma_signal`; gateway `sma_signal_cmd` + wiring =
   review-by-reading. verify=`[cargo build, cargo test --test sma_signal]`; full-verify already in
   current.json.
3. Impl → omp (goal-driven-impl-claude); review → codex (check). Model: task/review frontier; impl capable-local OK.
