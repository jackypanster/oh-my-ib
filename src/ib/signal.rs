//! `signal` — the read-only Faber 200-day month-end timing seam (ADR 0034).
//!
//! Two layers:
//! - **Pure `sma_signal`** (FROZEN): offline, no ibapi. Takes a slice of `Bar { ym, close }` already
//!   stripped of timestamps, returns the standing HOLD/EXIT signal at the last COMPLETED month-end.
//!   This is the seam the frozen test pins.
//! - **Gateway `sma_signal_cmd`** (review-by-reading): resolves symbols (positions fallback), fetches
//!   2Y daily bars via `historical_data`, strips each `BarTimestamp` to `(year, month)` via `ym_of`,
//!   and emits the JSON envelope.
//!
//! READ-ONLY: no gate, no `place_order`/`cancel_order`, no live path (ADR 0017 does NOT apply —
//! this is not a write module). Default paper port.

use ibapi::market_data::historical::{BarSize, BarTimestamp, Duration, WhatToShow};
use ibapi::prelude::Contract;
use serde_json::{json, Value};
use time::Date;

use crate::cli::SmaSignalArgs;
use crate::config::Config;
use crate::error::AppError;

/// The standing Faber signal at the last completed month-end.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalState {
    /// Month-end close ≥ its n-day SMA — hold the position.
    Hold,
    /// Month-end close < its n-day SMA — exit the position.
    Exit,
    /// Not enough history to compute the SMA as of the month-end.
    Insufficient,
}

impl SignalState {
    /// Machine-readable string for the JSON envelope (form chosen by the gateway; the frozen test
    /// compares the enum, not this string).
    pub fn as_str(self) -> &'static str {
        match self {
            SignalState::Hold => "HOLD",
            SignalState::Exit => "EXIT",
            SignalState::Insufficient => "INSUFFICIENT",
        }
    }
}

/// A daily bar stripped to the (year, month) bucket and the close. `ym` compares as a plain tuple
/// (`(i32, u32)` lexicographic = chronological) — the pure seam never touches a real date.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bar {
    pub ym: (i32, u32),
    pub close: f64,
}

/// The Faber 200-day month-end signal result. `as_of_month_end` is the (year, month) the signal is
/// evaluated at; the `latest_*` fields are the current-bar drift vs the n-day SMA ending there.
#[derive(Debug, Clone, PartialEq)]
pub struct SmaSignal {
    pub state: SignalState,
    pub as_of_month_end: (i32, u32),
    pub month_end_close: f64,
    pub sma: f64,
    pub distance_pct: f64,
    pub latest_close: f64,
    pub latest_sma: f64,
    pub latest_distance_pct: f64,
    pub bars_used: usize,
}

/// The zero value returned for every Insufficient path (numeric fields 0.0; `as_of_month_end`
/// set by the caller to either `(0,0)` for empty input or the last available month-end).
fn insufficient(ym: (i32, u32), bars_used: usize) -> SmaSignal {
    SmaSignal {
        state: SignalState::Insufficient,
        as_of_month_end: ym,
        month_end_close: 0.0,
        sma: 0.0,
        distance_pct: 0.0,
        latest_close: 0.0,
        latest_sma: 0.0,
        latest_distance_pct: 0.0,
        bars_used,
    }
}

/// Pure Faber month-end timing signal (ADR 0034 D-RULE). Deterministic from inputs — the frozen
/// surface. See the module docs and the card's algorithm block for the exact selection rule.
///
/// In short: pick `as_of_idx` = the last bar whose month is strictly before the in-progress final
/// month (the last COMPLETED month-end); if no such bar exists (single-month series), use the final
/// bar. The signal is HOLD if that month-end close ≥ its trailing-n-day SMA, else EXIT. If there
/// aren't `n` bars up to and including `as_of_idx` (or the series is shorter than `n`), it's
/// INSUFFICIENT. `latest_*` always reflects the final bar (the series is ≥ n once non-insufficient).
pub fn sma_signal(bars: &[Bar], n: usize) -> SmaSignal {
    let bars_used = bars.len();

    // Empty series — nothing to evaluate.
    if bars.is_empty() {
        return insufficient((0, 0), 0);
    }
    // A non-positive SMA window is nonsensical; treat it as Insufficient so the pure seam is
    // total and can never reach `sma_at` with a window that would slice start>end (n==0 ⇒
    // start = i+1 > i). Also covers `bars.len() < n` — can't form even one n-day SMA.
    if n == 0 || bars.len() < n {
        return insufficient(bars[bars.len() - 1].ym, bars_used);
    }

    // The in-progress final month is excluded; the signal stands at the last COMPLETED month-end.
    // If the whole series is one month, there is no earlier month — evaluate at the final bar.
    let last_ym = bars[bars.len() - 1].ym;
    let as_of_idx = match bars.iter().rposition(|b| b.ym < last_ym) {
        Some(i) => i,
        None => bars.len() - 1,
    };

    // Not enough history up to the chosen month-end to form an n-day SMA.
    if as_of_idx + 1 < n {
        return insufficient(bars[as_of_idx].ym, bars_used);
    }

    // n-day SMA ending at as_of_idx (the window is bars[as_of_idx+1-n ..= as_of_idx]).
    let sma = sma_at(bars, as_of_idx, n);
    let month_end_close = bars[as_of_idx].close;
    let state = if month_end_close >= sma {
        SignalState::Hold
    } else {
        SignalState::Exit
    };
    let distance_pct = (month_end_close - sma) / sma * 100.0;

    // Latest-bar drift. len >= n and li = len-1 ⇒ li+1 >= n holds, so sma_at is defined.
    let li = bars.len() - 1;
    let latest_close = bars[li].close;
    let latest_sma = sma_at(bars, li, n);
    let latest_distance_pct = (latest_close - latest_sma) / latest_sma * 100.0;

    SmaSignal {
        state,
        as_of_month_end: bars[as_of_idx].ym,
        month_end_close,
        sma,
        distance_pct,
        latest_close,
        latest_sma,
        latest_distance_pct,
        bars_used,
    }
}

/// Mean of the `n` closes ending at `i` (window `bars[i+1-n ..= i]`). Caller guarantees `i+1 >= n`
/// and `i < bars.len()`; the pure seam only calls this after those guards.
fn sma_at(bars: &[Bar], i: usize, n: usize) -> f64 {
    let start = i + 1 - n;
    let sum: f64 = bars[start..=i].iter().map(|b| b.close).sum();
    sum / n as f64
}

// ---- gateway driver (review-by-reading, NOT frozen) ------------------------

/// Strip a `BarTimestamp` to its `(year, month)` bucket for the pure planner. Day bars arrive as
/// `Date`; intraday as `OffsetDateTime` — both carry the calendar accessors we need.
fn ym_of(ts: &BarTimestamp) -> (i32, u32) {
    match ts {
        BarTimestamp::Date(d) => ym_of_date(*d),
        BarTimestamp::DateTime(dt) => (dt.year(), u8::from(dt.month()) as u32),
    }
}

fn ym_of_date(d: Date) -> (i32, u32) {
    (d.year(), u8::from(d.month()) as u32)
}

/// The gateway command: resolve symbols (positions fallback when none given), fetch 2Y of daily
/// bars per symbol, run the pure signal, emit the JSON envelope. Read-only — no gate, no writes.
pub fn sma_signal_cmd(cfg: &Config, args: &SmaSignalArgs) -> Result<Value, AppError> {
    // Validate the SMA window BEFORE any gateway work (held_symbols/connect) so a bad --sma is a
    // structured config error (exit 5), not a silent panic deep in the pure seam.
    if args.sma < 1 {
        return Err(AppError::config(
            format!("--sma must be >= 1, got {}", args.sma),
            "sma-signal",
        ));
    }
    let syms = if args.symbols.is_empty() {
        held_symbols(cfg)?
    } else {
        args.symbols.clone()
    };

    let client = super::connect(cfg)?;
    let mut signals: Vec<Value> = Vec::with_capacity(syms.len());
    for sym in &syms {
        let contract = Contract::stock(sym.as_str()).build();
        let data = client
            .historical_data(&contract, BarSize::Day)
            .what_to_show(WhatToShow::Trades)
            .duration(Duration::years(2))
            .fetch()
            .map_err(|e| AppError::data(format!("historical_data failed: {e}"), "sma-signal"))?;
        let bars: Vec<Bar> = data
            .bars
            .iter()
            .map(|b| Bar {
                ym: ym_of(&b.date),
                close: b.close,
            })
            .collect();
        let s = sma_signal(&bars, args.sma);
        signals.push(json!({
            "symbol": sym,
            "state": s.state.as_str(),
            "as_of": format!("{:04}-{:02}", s.as_of_month_end.0, s.as_of_month_end.1),
            "month_end_close": s.month_end_close,
            "sma": s.sma,
            "distance_pct": s.distance_pct,
            "latest_close": s.latest_close,
            "latest_sma": s.latest_sma,
            "latest_distance_pct": s.latest_distance_pct,
            "bars_used": s.bars_used,
        }));
    }
    Ok(json!({ "signals": signals }))
}

/// Resolve the held-position symbols when the operator runs `omi sma-signal` with no args.
/// Reuses the read-only `positions` command's drain (account_updates), extracting just the
/// `symbol` field from each portfolio row.
fn held_symbols(cfg: &Config) -> Result<Vec<String>, AppError> {
    let val = super::positions(cfg)?;
    let mut out = Vec::new();
    if let Some(rows) = val["positions"].as_array() {
        for r in rows {
            if let Some(s) = r["symbol"].as_str() {
                out.push(s.to_string());
            }
        }
    }
    Ok(out)
}
