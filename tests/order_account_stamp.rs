//! FROZEN SPEC — order-account-stamp card 01 (ADR 0024). Offline. The coder must NOT edit
//! this file.
//!
//! Freezes the pure stamping seam `stamp_order_account(&mut Order, &str)`: sets
//! `order.account` (OVERWRITING any prior value — the resolved account is the only
//! authority) and touches NOTHING else. "Nothing else" is asserted in the strongest form:
//! whole-struct equality against an expected Order that differs only in `account`
//! (ibapi Order derives PartialEq). Covered for both LMT and MKT shapes.
//! RED until impl re-exports `oh_my_ib::ib::stamp_order_account`.
//!
//! NOT frozen (reviewed-by-reading + paper acceptance, PRD criterion 7): the choke-point
//! wiring — place_with_client's REQUIRED &AccountId param, clone-then-stamp before
//! place_order, place_core/option_combo resolving after connect, option_close passing its
//! already-resolved account — and the AGENTS.md phrase. The pure builders still emit
//! account="" (their frozen suites are untouched by this feature).

use ibapi::orders::{Action, Order, TimeInForce};

use oh_my_ib::ib::stamp_order_account;

fn lmt_order() -> Order {
    Order {
        action: Action::Buy,
        total_quantity: 2.0,
        order_type: "LMT".into(),
        limit_price: Some(5.5),
        tif: TimeInForce::Day,
        ..Default::default()
    }
}

fn mkt_order() -> Order {
    Order {
        action: Action::Sell,
        total_quantity: 100.0,
        order_type: "MKT".into(),
        tif: TimeInForce::Day,
        ..Default::default()
    }
}

#[test]
fn stamps_the_account_onto_a_default_empty_order() {
    let mut order = lmt_order();
    assert_eq!(order.account, "", "builders emit empty account (frozen elsewhere)");
    stamp_order_account(&mut order, "DU1234567");
    assert_eq!(order.account, "DU1234567");
}

#[test]
fn overwrites_a_preset_account_resolved_is_the_only_authority() {
    let mut order = lmt_order();
    order.account = "STALE".to_string();
    stamp_order_account(&mut order, "DU1234567");
    assert_eq!(order.account, "DU1234567", "no respect-existing arm (ADR 0024 §2)");
}

#[test]
fn lmt_order_is_otherwise_byte_identical() {
    let mut stamped = lmt_order();
    stamp_order_account(&mut stamped, "DU1234567");
    let mut expected = lmt_order();
    expected.account = "DU1234567".to_string();
    assert_eq!(stamped, expected, "account is the ONLY field the seam may touch");
}

#[test]
fn mkt_order_is_otherwise_byte_identical_and_limit_stays_none() {
    let mut stamped = mkt_order();
    stamp_order_account(&mut stamped, "DU1234567");
    assert_eq!(stamped.limit_price, None, "MKT shape preserved");
    let mut expected = mkt_order();
    expected.account = "DU1234567".to_string();
    assert_eq!(stamped, expected);
}

#[test]
fn empty_account_string_stamps_as_empty_degenerate_totality() {
    // Unreachable in the verb (resolve_account errors before an empty id) — seam is total.
    let mut order = lmt_order();
    order.account = "STALE".to_string();
    stamp_order_account(&mut order, "");
    assert_eq!(order.account, "");
}
