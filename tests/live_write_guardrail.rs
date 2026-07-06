//! FROZEN SPEC — live-write-guardrail (card 01): the four pure decision seams of the live write
//! posture guardrail (ADR 0030). Offline. The coder must NOT edit this file.
//!
//! Freezes:
//!
//! - `compute_notional(quantity, limit: Option<f64>, multiplier) -> Option<f64>` — `qty × |limit| ×
//!   mult`; MKT (`None` limit) ⇒ `None`. Mirrors the `shape_preview` math.
//! - `resolve_max_notional(raw: Option<&str>) -> Result<f64, String>` — `None` ⇒ the $500 default;
//!   `Some` ⇒ parse a finite, strictly-positive `f64`, else `Err` (fail-closed).
//! - `check_live_write_posture(is_live, is_mkt, notional, cap) -> Result<(), String>` — paper
//!   (`is_live == false`) is always `Ok`; live MKT ⇒ `Err`; live notional `> cap` ⇒ `Err`; live
//!   within-cap LMT ⇒ `Ok` (boundary `notional == cap` is `Ok` — `>` is the refuse, not `>=`).
//! - `refuse_live_combo_on_live(is_live) -> Result<(), String>` — live ⇒ `Err` (combo is paper-only);
//!   paper ⇒ `Ok`.
//!
//! RED until impl adds the four seams to `src/ib/trade.rs` and re-exports them at `src/ib/mod.rs`
//! (the import below will not resolve until then).
//!
//! NOT frozen (reviewed-by-reading, ADR 0030 §Seams): the `place_core` / `option_combo` wiring
//! (`is_live`/`multiplier`/`is_mkt`/`notional` derivation; gate → posture → connect ordering; combo
//! refuse before the gate on the real path; `AppError::config` mapping). NOT frozen (operator live
//! acceptance): a live refuse actually exits 5 with no order, and the within-cap first trial order
//! actually places. Paper/preview/close/cancel exemption is a wiring property, not asserted here.

use oh_my_ib::ib::{
    check_live_write_posture, compute_notional, refuse_live_combo_on_live, resolve_max_notional,
};

// ---- compute_notional -------------------------------------------------------

#[test]
fn compute_notional_option_uses_multiplier_100() {
    // 2 contracts × $3.00 × 100 = $600.
    assert_eq!(compute_notional(2.0, Some(3.0), 100.0), Some(600.0));
}

#[test]
fn compute_notional_stock_multiplier_1() {
    // 100 shares × $250 × 1 = $25,000.
    assert_eq!(compute_notional(100.0, Some(250.0), 1.0), Some(25_000.0));
}

#[test]
fn compute_notional_uses_absolute_limit() {
    // A combo net credit (negative limit) still yields a positive notional: 1 × |−0.50| × 100 = 50.
    assert_eq!(compute_notional(1.0, Some(-0.50), 100.0), Some(50.0));
}

#[test]
fn compute_notional_mkt_is_none() {
    assert_eq!(compute_notional(100.0, None, 1.0), None);
}

// ---- resolve_max_notional (fail-closed) ------------------------------------

#[test]
fn resolve_max_notional_absent_is_default_500() {
    assert_eq!(resolve_max_notional(None), Ok(500.0));
}

#[test]
fn resolve_max_notional_parses_positive() {
    assert_eq!(resolve_max_notional(Some("1000")), Ok(1000.0));
    assert_eq!(resolve_max_notional(Some("1000.5")), Ok(1000.5));
}

#[test]
fn resolve_max_notional_rejects_non_numeric() {
    assert!(resolve_max_notional(Some("abc")).is_err());
    assert!(resolve_max_notional(Some("")).is_err());
}

#[test]
fn resolve_max_notional_rejects_non_positive() {
    assert!(resolve_max_notional(Some("0")).is_err());
    assert!(resolve_max_notional(Some("-5")).is_err());
}

#[test]
fn resolve_max_notional_rejects_non_finite() {
    // finite required — "inf"/"nan" parse to f64 but must be refused (fail-closed).
    assert!(resolve_max_notional(Some("inf")).is_err());
    assert!(resolve_max_notional(Some("nan")).is_err());
}

// ---- check_live_write_posture ----------------------------------------------

#[test]
fn posture_paper_is_always_ok() {
    // Paper (is_live=false) is exempt: MKT ok, and any magnitude ok.
    assert!(check_live_write_posture(false, true, None, 500.0).is_ok());
    assert!(check_live_write_posture(false, false, Some(1_000_000_000.0), 500.0).is_ok());
}

#[test]
fn posture_live_mkt_is_refused() {
    assert!(check_live_write_posture(true, true, None, 500.0).is_err());
}

#[test]
fn posture_live_over_cap_is_refused() {
    assert!(check_live_write_posture(true, false, Some(600.0), 500.0).is_err());
}

#[test]
fn posture_live_within_cap_is_ok() {
    assert!(check_live_write_posture(true, false, Some(300.0), 500.0).is_ok());
}

#[test]
fn posture_live_boundary_equal_cap_is_ok() {
    // `>` is the refuse, not `>=` — notional exactly at the cap passes.
    assert!(check_live_write_posture(true, false, Some(500.0), 500.0).is_ok());
}

// ---- refuse_live_combo_on_live ---------------------------------------------

#[test]
fn combo_refused_on_live_allowed_on_paper() {
    assert!(refuse_live_combo_on_live(true).is_err());
    assert!(refuse_live_combo_on_live(false).is_ok());
}
