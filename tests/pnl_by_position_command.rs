//! FROZEN SPEC — pnl-by-position (card 01). Offline. The coder must NOT edit this file.
//!
//! Freezes the black-box CLI contract of `omi pnl-by-position` + the pure shaping seam
//! `shape_pnl_by_position` (per-position rows → JSON array, money fields through the `pnl_number`
//! sentinel seam). RED until impl adds the `PnlByPosition` subcommand and re-exports
//! `oh_my_ib::ib::{shape_pnl_by_position, PnlSingleRow}` (the import below won't resolve, and
//! `--help` won't list `pnl-by-position`, until then).
//!
//! NOT frozen (reviewed-by-reading + operator live acceptance on :4001, PRD D3 gate): the
//! `pnl_by_position(cfg)` gateway fn — discovery via `account_updates` drain-to-End, the N
//! sequential `pnl_single` take-first reads (ADR 0009 — markerless stream, NOT drain-to-End;
//! fail-fast, no partial sweep), account resolution, `{account, by_position:[…]}` assembly, and
//! `--format table`.

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::{json, Value};

use oh_my_ib::ib::{shape_pnl_by_position, PnlSingleRow};

/// IB "no value" marker: Double.MAX_VALUE == f64::MAX.
const SENTINEL: f64 = 1.7976931348623157e308;

fn omi() -> Command {
    Command::cargo_bin("omi").expect("the `omi` binary should build")
}

// ---- fixtures (plain rows; NO ibapi types) ----

fn row(conid: i32, symbol: &str) -> PnlSingleRow {
    PnlSingleRow {
        conid,
        symbol: symbol.into(),
        position: 100.0,
        daily_pnl: 52.3,
        unrealized_pnl: 1204.5,
        realized_pnl: -3.25,
        value: 21050.0,
    }
}

// ---- black-box CLI contract ----

#[test]
fn help_lists_pnl_by_position_subcommand() {
    omi()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("pnl-by-position"));
}

#[test]
fn pnl_by_position_help_succeeds() {
    omi().args(["pnl-by-position", "--help"]).assert().success();
}

// ---- pure shaping seam (rows → JSON array; sentinel → null) ----

#[test]
fn finite_row_passes_through() {
    let out = shape_pnl_by_position(vec![row(265598, "AAPL")]);
    let arr = out.as_array().expect("by_position must be a JSON array");
    assert_eq!(arr.len(), 1);
    let r = &arr[0];
    assert_eq!(r["conid"], json!(265598));
    assert_eq!(r["symbol"], json!("AAPL"));
    assert_eq!(r["position"], json!(100.0));
    assert_eq!(r["daily_pnl"], json!(52.3));
    assert_eq!(r["unrealized_pnl"], json!(1204.5));
    assert_eq!(r["realized_pnl"], json!(-3.25));
    assert_eq!(r["value"], json!(21050.0));
}

#[test]
fn row_has_exactly_the_seven_contract_keys() {
    let out = shape_pnl_by_position(vec![row(1, "X")]);
    let obj = out.as_array().unwrap()[0]
        .as_object()
        .expect("each row must be a JSON object");
    let mut keys: Vec<&str> = obj.keys().map(|k| k.as_str()).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        ["conid", "daily_pnl", "position", "realized_pnl", "symbol", "unrealized_pnl", "value"]
    );
}

#[test]
fn sentinel_maps_to_null_in_every_money_field() {
    // The sentinel arrives as a VALUE (PnLSingle fields are bare f64, not Option) — every money
    // field must pass through pnl_number; identity fields must NOT be touched by the seam.
    let mut r = row(265598, "AAPL");
    r.daily_pnl = SENTINEL;
    r.unrealized_pnl = SENTINEL;
    r.realized_pnl = SENTINEL;
    r.value = SENTINEL;
    let out = shape_pnl_by_position(vec![r]);
    let row0 = &out.as_array().unwrap()[0];
    assert_eq!(row0["daily_pnl"], Value::Null);
    assert_eq!(row0["unrealized_pnl"], Value::Null);
    assert_eq!(row0["realized_pnl"], Value::Null);
    assert_eq!(row0["value"], Value::Null);
    assert_eq!(row0["conid"], json!(265598));
    assert_eq!(row0["symbol"], json!("AAPL"));
    assert_eq!(row0["position"], json!(100.0));
}

#[test]
fn non_finite_maps_to_null() {
    let mut r = row(2, "NANCO");
    r.daily_pnl = f64::NAN;
    r.unrealized_pnl = f64::INFINITY;
    r.value = f64::NEG_INFINITY;
    let out = shape_pnl_by_position(vec![r]);
    let row0 = &out.as_array().unwrap()[0];
    assert_eq!(row0["daily_pnl"], Value::Null);
    assert_eq!(row0["unrealized_pnl"], Value::Null);
    assert_eq!(row0["value"], Value::Null);
    // a finite sibling field is untouched
    assert_eq!(row0["realized_pnl"], json!(-3.25));
}

#[test]
fn zero_position_row_is_emitted_not_filtered() {
    // PRD D6: closed-today (qty 0) rows carry today's realized PnL — the seam must never filter
    // them; position passes through raw (0 is data, not absence).
    let mut r = row(3, "CLOSED");
    r.position = 0.0;
    r.realized_pnl = 87.5;
    let out = shape_pnl_by_position(vec![r]);
    let arr = out.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["position"], json!(0.0));
    assert_eq!(arr[0]["realized_pnl"], json!(87.5));
}

#[test]
fn order_preserved() {
    let out = shape_pnl_by_position(vec![row(1, "FIRST"), row(2, "SECOND")]);
    let arr = out.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["conid"], json!(1));
    assert_eq!(arr[0]["symbol"], json!("FIRST"));
    assert_eq!(arr[1]["conid"], json!(2));
    assert_eq!(arr[1]["symbol"], json!("SECOND"));
}

#[test]
fn empty_input_is_empty_array() {
    assert_eq!(shape_pnl_by_position(vec![]), json!([]));
}
