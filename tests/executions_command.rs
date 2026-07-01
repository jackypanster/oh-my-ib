//! FROZEN SPEC — executions-command (card 01). Offline. The coder must NOT edit this file.
//!
//! Freezes the black-box CLI contract of `omi executions` + the pure JOIN seam `merge_executions`
//! (exec↔commission by `exec_id`). RED until impl adds the `Executions` subcommand and re-exports
//! `oh_my_ib::ib::{merge_executions, ExecRow, CommissionRow}` (the import below won't resolve, and
//! `--help` won't list `executions`, until then).
//!
//! NOT frozen (reviewed-by-reading + operator live acceptance on :4001): the `client.executions()`
//! wiring, the drain-to-End read via `iter_data()` (ADR 0008 — ExecutionDataEnd → EndOfStream; NOT the
//! reqPnL take-first of ADR 0007), the ibapi-item→row extraction, account resolution, JSON assembly,
//! and `--format table`.

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::{json, Value};

use oh_my_ib::ib::{merge_executions, CommissionRow, ExecRow};

fn omi() -> Command {
    Command::cargo_bin("omi").expect("the `omi` binary should build")
}

// ---- fixtures (plain rows; NO ibapi types) ----

fn exec(exec_id: &str, side: &str) -> ExecRow {
    ExecRow {
        exec_id: exec_id.into(),
        order_id: 12,
        perm_id: 987654321,
        time: "20260701 09:31:07 US/Eastern".into(),
        symbol: "AAPL".into(),
        conid: 265598,
        side: side.into(),
        shares: 100.0,
        price: 189.32,
        cumulative_qty: 100.0,
        avg_price: 189.30,
        exchange: "NASDAQ".into(),
    }
}

fn comm(exec_id: &str, realized_pnl: Option<f64>) -> CommissionRow {
    CommissionRow {
        exec_id: exec_id.into(),
        commission: 1.25,
        currency: "USD".into(),
        realized_pnl,
    }
}

// ---- black-box CLI contract ----

#[test]
fn help_lists_executions_subcommand() {
    omi()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("executions"));
}

#[test]
fn executions_help_succeeds() {
    omi().args(["executions", "--help"]).assert().success();
}

// ---- pure JOIN seam (exec↔commission by exec_id) ----

#[test]
fn matched_execution_joins_commission() {
    let out = merge_executions(vec![exec("e1", "BOT")], vec![comm("e1", Some(12.5))]);
    let arr = out.as_array().expect("executions must be a JSON array");
    assert_eq!(arr.len(), 1);
    let row = &arr[0];
    // passthrough exec fields
    assert_eq!(row["exec_id"], json!("e1"));
    assert_eq!(row["side"], json!("BOT"));
    assert_eq!(row["symbol"], json!("AAPL"));
    assert_eq!(row["price"], json!(189.32));
    // joined commission fields
    assert_eq!(row["commission"], json!(1.25));
    assert_eq!(row["commission_currency"], json!("USD"));
    assert_eq!(row["realized_pnl"], json!(12.5));
}

#[test]
fn execution_without_commission_has_null_fields() {
    let out = merge_executions(vec![exec("e1", "SLD")], vec![]);
    let row = &out.as_array().unwrap()[0];
    assert_eq!(row["side"], json!("SLD")); // row itself still complete
    assert_eq!(row["commission"], Value::Null);
    assert_eq!(row["commission_currency"], Value::Null);
    assert_eq!(row["realized_pnl"], Value::Null);
}

#[test]
fn realized_pnl_sentinel_and_none_are_null() {
    // IB "no value" sentinel Double.MAX_VALUE (1.7976931348623157e308) → null (via pnl_number reuse),
    // even though a commission report IS present.
    let sent = merge_executions(
        vec![exec("e1", "BOT")],
        vec![comm("e1", Some(1.7976931348623157e308))],
    );
    assert_eq!(sent.as_array().unwrap()[0]["realized_pnl"], Value::Null);

    // realized_pnl None → null; the commission amount itself is still reported.
    let none = merge_executions(vec![exec("e2", "BOT")], vec![comm("e2", None)]);
    let row = &none.as_array().unwrap()[0];
    assert_eq!(row["realized_pnl"], Value::Null);
    assert_eq!(row["commission"], json!(1.25));
}

#[test]
fn order_preserved_and_side_verbatim() {
    let out = merge_executions(vec![exec("first", "BOT"), exec("second", "SLD")], vec![]);
    let arr = out.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["exec_id"], json!("first"));
    assert_eq!(arr[0]["side"], json!("BOT"));
    assert_eq!(arr[1]["exec_id"], json!("second"));
    assert_eq!(arr[1]["side"], json!("SLD"));
}

#[test]
fn orphan_commission_dropped_and_joined_by_exec_id() {
    // one execution; two commissions, one of which references no execution → exactly one row,
    // joined to the matching commission by exec_id (the orphan produces no phantom row).
    let out = merge_executions(
        vec![exec("e1", "BOT")],
        vec![comm("orphan", Some(9.9)), comm("e1", Some(3.3))],
    );
    let arr = out.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["exec_id"], json!("e1"));
    assert_eq!(arr[0]["realized_pnl"], json!(3.3));
}

#[test]
fn empty_input_is_empty_array() {
    assert_eq!(merge_executions(vec![], vec![]), json!([]));
}
