//! FROZEN SPEC — pnl-command (card 01). Offline. The coder must NOT edit this file.
//!
//! Freezes the black-box CLI contract of `omi pnl` + the pure sentinel filter `pnl_number`.
//! RED until impl adds the `Pnl` subcommand and re-exports `oh_my_ib::ib::pnl_number`
//! (the import below won't resolve, and `--help` won't list `pnl`, until then).
//!
//! NOT frozen (reviewed-by-reading + operator live acceptance on :4001): the `client.pnl()` wiring,
//! the take-first `Subscription::next_data()` read (ADR 0007 — reqPnL has no End marker), account
//! resolution, JSON assembly {account,daily_pnl,unrealized_pnl,realized_pnl}, and `--format table`.

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::{json, Value};

use oh_my_ib::ib::pnl_number;

fn omi() -> Command {
    Command::cargo_bin("omi").expect("the `omi` binary should build")
}

// ---- black-box CLI contract ----

#[test]
fn help_lists_pnl_subcommand() {
    omi()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("pnl"));
}

#[test]
fn pnl_help_succeeds() {
    omi().args(["pnl", "--help"]).assert().success();
}

// ---- pure sentinel filter (operator decision A: sentinel/non-finite -> null) ----

#[test]
fn real_value_is_a_number() {
    assert_eq!(pnl_number(Some(123.45)), json!(123.45));
    assert_eq!(pnl_number(Some(-987.6)), json!(-987.6));
    assert_eq!(pnl_number(Some(0.0)), json!(0.0));
}

#[test]
fn ib_unset_sentinel_is_null() {
    // IBKR encodes "no value" as Double.MAX_VALUE == f64::MAX (1.7976931348623157e308).
    assert_eq!(pnl_number(Some(1.7976931348623157e308)), Value::Null);
    assert_eq!(pnl_number(Some(f64::MAX)), Value::Null);
}

#[test]
fn non_finite_is_null() {
    assert_eq!(pnl_number(Some(f64::INFINITY)), Value::Null);
    assert_eq!(pnl_number(Some(f64::NEG_INFINITY)), Value::Null);
    assert_eq!(pnl_number(Some(f64::NAN)), Value::Null);
}

#[test]
fn absent_is_null() {
    assert_eq!(pnl_number(None), Value::Null);
}
