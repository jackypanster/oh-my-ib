//! FROZEN SPEC — live-combo-unlock (card 01): the pure defined-risk seam that replaces the ADR 0030
//! combo lockout (ADR 0031). Offline. The coder must NOT edit this file.
//!
//! Freezes `combo_live_max_risk(specs: &[LegSpec], qty: f64) -> Result<f64, String>` — the live
//! combo breaker. It admits ONLY a clean 2-leg 1:1 vertical spread (exactly 2 legs, same underlying,
//! same expiry, same right, opposite actions, both ratio 1, distinct strikes) and returns its
//! PURE-WIDTH max risk `|strike_a - strike_b| * 100 * qty`. It is premium-proof: the net limit is
//! NOT a parameter, so a mistyped credit cannot widen the admitted risk. Every non-vertical shape
//! returns `Err(reason)` so the caller refuses it (stays paper-only). A credit and a debit vertical
//! of the same strikes yield the SAME risk (width is sign-free) — that equality is asserted below.
//!
//! RED until impl adds `combo_live_max_risk` to `src/ib/trade.rs` and re-exports it at
//! `src/ib/mod.rs` (the import below will not resolve until then).
//!
//! NOT frozen (reviewed-by-reading, ADR 0031 §Freeze-coverage): the `option_combo` wiring (`is_live`
//! derivation; `combo_live_max_risk(&specs, args.qty)` before connect; `resolve_max_notional` env
//! read; `check_live_write_posture(true, false, Some(risk), cap)`; posture-before-gate ordering;
//! every `Err -> AppError::config` exit 5; paper/preview and the now-unwired
//! `refuse_live_combo_on_live` unchanged). NOT frozen (operator live acceptance): a within-cap
//! vertical actually places, an over-cap / non-vertical live combo refuses exit 5 with no order.

use oh_my_ib::ib::{combo_live_max_risk, LegSpec};

/// Build one normalized leg (action `BUY`/`SELL`, right `C`/`P`, expiry `YYYYMMDD`) — mirrors what
/// `parse_combo_leg` produces, so the frozen inputs match the real call site.
fn leg(action: &str, ratio: i32, symbol: &str, expiry: &str, strike: f64, right: &str) -> LegSpec {
    LegSpec {
        action: action.to_string(),
        ratio,
        symbol: symbol.to_string(),
        expiry: expiry.to_string(),
        strike,
        right: right.to_string(),
    }
}

// ---- Ok: clean 2-leg 1:1 verticals, pure-width risk ------------------------

#[test]
fn put_credit_vertical_qty1_is_width_times_100() {
    // Sell 185P / Buy 180P, same expiry, 1 lot: width 5 * 100 * 1 = 500. (The motivating NVDA order.)
    let legs = [
        leg("SELL", 1, "NVDA", "20260715", 185.0, "P"),
        leg("BUY", 1, "NVDA", "20260715", 180.0, "P"),
    ];
    assert_eq!(combo_live_max_risk(&legs, 1.0), Ok(500.0));
}

#[test]
fn call_vertical_qty_scales_risk() {
    // Buy 250C / Sell 240C, 2 lots: width 10 * 100 * 2 = 2000.
    let legs = [
        leg("BUY", 1, "NVDA", "20260715", 250.0, "C"),
        leg("SELL", 1, "NVDA", "20260715", 240.0, "C"),
    ];
    assert_eq!(combo_live_max_risk(&legs, 2.0), Ok(2000.0));
}

#[test]
fn debit_vertical_equals_credit_vertical_premium_proof() {
    // Same strikes as the credit case, actions flipped (a debit spread). Width is sign-free, so the
    // risk is identical (500) — the seam never reads the premium.
    let legs = [
        leg("BUY", 1, "NVDA", "20260715", 180.0, "P"),
        leg("SELL", 1, "NVDA", "20260715", 185.0, "P"),
    ];
    assert_eq!(combo_live_max_risk(&legs, 1.0), Ok(500.0));
}

// ---- Err: every non-vertical shape stays refused ---------------------------

#[test]
fn one_leg_is_refused() {
    let legs = [leg("SELL", 1, "NVDA", "20260715", 185.0, "P")];
    assert!(combo_live_max_risk(&legs, 1.0).is_err());
}

#[test]
fn three_legs_is_refused() {
    let legs = [
        leg("SELL", 1, "NVDA", "20260715", 185.0, "P"),
        leg("BUY", 1, "NVDA", "20260715", 180.0, "P"),
        leg("BUY", 1, "NVDA", "20260715", 175.0, "P"),
    ];
    assert!(combo_live_max_risk(&legs, 1.0).is_err());
}

#[test]
fn different_expiry_calendar_is_refused() {
    let legs = [
        leg("SELL", 1, "NVDA", "20260715", 185.0, "P"),
        leg("BUY", 1, "NVDA", "20260821", 180.0, "P"),
    ];
    assert!(combo_live_max_risk(&legs, 1.0).is_err());
}

#[test]
fn different_right_is_refused() {
    let legs = [
        leg("SELL", 1, "NVDA", "20260715", 185.0, "P"),
        leg("BUY", 1, "NVDA", "20260715", 180.0, "C"),
    ];
    assert!(combo_live_max_risk(&legs, 1.0).is_err());
}

#[test]
fn same_action_not_a_spread_is_refused() {
    let legs = [
        leg("BUY", 1, "NVDA", "20260715", 185.0, "P"),
        leg("BUY", 1, "NVDA", "20260715", 180.0, "P"),
    ];
    assert!(combo_live_max_risk(&legs, 1.0).is_err());
}

#[test]
fn ratio_not_one_is_refused() {
    // A 2:1 ratio/backspread is not a defined-risk 1:1 vertical.
    let legs = [
        leg("SELL", 2, "NVDA", "20260715", 185.0, "P"),
        leg("BUY", 1, "NVDA", "20260715", 180.0, "P"),
    ];
    assert!(combo_live_max_risk(&legs, 1.0).is_err());
}

#[test]
fn equal_strikes_zero_width_is_refused() {
    let legs = [
        leg("SELL", 1, "NVDA", "20260715", 185.0, "P"),
        leg("BUY", 1, "NVDA", "20260715", 185.0, "P"),
    ];
    assert!(combo_live_max_risk(&legs, 1.0).is_err());
}

#[test]
fn different_underlying_is_refused() {
    let legs = [
        leg("SELL", 1, "NVDA", "20260715", 185.0, "P"),
        leg("BUY", 1, "AAPL", "20260715", 180.0, "P"),
    ];
    assert!(combo_live_max_risk(&legs, 1.0).is_err());
}
