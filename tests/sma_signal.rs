//! FROZEN SPEC — sma-signal (ADR 0034): the pure Faber 200-day month-end timing seam.
//! Offline. The coder must NOT edit this file.
//!
//! Freezes `sma_signal(bars: &[Bar], n) -> SmaSignal`. Per a series of daily bars (each already
//! stripped to `Bar { ym: (year, month), close }`, ascending by date), it evaluates the standing
//! signal at the LAST COMPLETED month-end (the last trading day of the last calendar month strictly
//! before the in-progress final month): `HOLD` if that month-end close ≥ its n-day SMA, `EXIT` if
//! below, `INSUFFICIENT` if there is not enough history to compute the SMA as of that month-end. It
//! also reports the latest-bar drift (`latest_*`). Tests use n=3 with hand-built arrays so every SMA
//! is exact and hand-checkable.
//!
//! RED until impl adds `src/ib/signal.rs` (`pub mod`-exported at `src/ib/mod.rs`) with the Bar /
//! SmaSignal / SignalState types + `sma_signal` — the `oh_my_ib::ib::sma_signal` import won't resolve
//! until then.
//!
//! NOT frozen (review-by-reading, ADR 0034 §Freeze-coverage): the gateway `sma_signal_cmd` (symbol
//! resolution incl. no-args→positions, the `historical_data` fetch, the `BarTimestamp → (y,m)` strip,
//! JSON shape), the read-only posture (no gate / no write symbols), the CLI wiring.

use oh_my_ib::ib::{sma_signal, Bar, SignalState};

fn bar(y: i32, m: u32, close: f64) -> Bar {
    Bar { ym: (y, m), close }
}

fn approx(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

// (A) last completed month-end close ABOVE its SMA ⇒ HOLD; as-of is the PRIOR month (not the
//     in-progress final month); SMA computed AS OF that month-end; latest_* reflect the final bar.
#[test]
fn month_end_above_sma_is_hold_and_uses_last_completed_month() {
    // Jan 2024: 10,11,12,15 (month-end = the 15).  Feb 2024 (in progress): 9,8 (latest = 8).
    let bars = [
        bar(2024, 1, 10.0), bar(2024, 1, 11.0), bar(2024, 1, 12.0), bar(2024, 1, 15.0),
        bar(2024, 2, 9.0), bar(2024, 2, 8.0),
    ];
    let s = sma_signal(&bars, 3);
    assert_eq!(s.state, SignalState::Hold);
    assert_eq!(s.as_of_month_end, (2024, 1), "signal is evaluated at the last COMPLETED month (Jan)");
    assert!(approx(s.month_end_close, 15.0));
    assert!(approx(s.sma, (11.0 + 12.0 + 15.0) / 3.0), "SMA of the 3 closes ending at the Jan month-end");
    assert!(approx(s.distance_pct, (15.0 - (38.0 / 3.0)) / (38.0 / 3.0) * 100.0));
    // drift: latest bar (Feb 8) vs SMA of the last 3 closes (15,9,8)
    assert!(approx(s.latest_close, 8.0));
    assert!(approx(s.latest_sma, (15.0 + 9.0 + 8.0) / 3.0));
    assert!(approx(s.latest_distance_pct, (8.0 - (32.0 / 3.0)) / (32.0 / 3.0) * 100.0));
    assert_eq!(s.bars_used, 6);
}

// (B) last completed month-end close BELOW its SMA ⇒ EXIT.
#[test]
fn month_end_below_sma_is_exit() {
    // Jan 2024: 20,18,16,10 (month-end = 10). Feb 2024 (in progress): 9.
    let bars = [
        bar(2024, 1, 20.0), bar(2024, 1, 18.0), bar(2024, 1, 16.0), bar(2024, 1, 10.0),
        bar(2024, 2, 9.0),
    ];
    let s = sma_signal(&bars, 3);
    assert_eq!(s.state, SignalState::Exit);
    assert_eq!(s.as_of_month_end, (2024, 1));
    assert!(approx(s.sma, (18.0 + 16.0 + 10.0) / 3.0));
    assert!(approx(s.distance_pct, (10.0 - (44.0 / 3.0)) / (44.0 / 3.0) * 100.0));
}

// (E) only ONE month present ⇒ evaluate at the final bar (no earlier month to fall back to).
#[test]
fn single_month_evaluates_at_final_bar() {
    let bars = [bar(2024, 1, 10.0), bar(2024, 1, 11.0), bar(2024, 1, 12.0)];
    let s = sma_signal(&bars, 3);
    assert_eq!(s.state, SignalState::Hold);
    assert_eq!(s.as_of_month_end, (2024, 1));
    assert!(approx(s.sma, 11.0));
    assert!(approx(s.distance_pct, (12.0 - 11.0) / 11.0 * 100.0));
    assert_eq!(s.bars_used, 3);
}

// (C) fewer than n bars total ⇒ INSUFFICIENT (no panic, no out-of-range slice).
#[test]
fn too_few_bars_is_insufficient() {
    let bars = [bar(2024, 1, 10.0), bar(2024, 1, 11.0)];
    let s = sma_signal(&bars, 3);
    assert_eq!(s.state, SignalState::Insufficient);
    assert_eq!(s.bars_used, 2);
}

// (D) enough bars overall, but not enough UP TO the last completed month-end ⇒ INSUFFICIENT.
#[test]
fn insufficient_history_up_to_month_end_is_insufficient() {
    // Jan has only 2 bars; the last completed month-end (Jan) can't carry a 3-day SMA.
    let bars = [
        bar(2024, 1, 10.0), bar(2024, 1, 11.0),
        bar(2024, 2, 12.0), bar(2024, 2, 13.0),
    ];
    let s = sma_signal(&bars, 3);
    assert_eq!(s.state, SignalState::Insufficient);
    assert_eq!(s.bars_used, 4);
}
