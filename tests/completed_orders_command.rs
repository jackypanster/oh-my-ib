//! FROZEN SPEC — completed-orders (card 01). Offline. The coder must NOT edit this file.
//!
//! Freezes the pure shaping seam `shape_completed_orders` (ADR 0015: rows in gateway order →
//! JSON array of exact 14-key objects — open-orders 10-key parity + 4 completion keys;
//! `limit_price`/`aux_price` None → null; "" completion strings pass through; empty ⇒ `[]`)
//! and the black-box CLI contract of `omi completed-orders`. RED until impl adds the
//! `CompletedOrders` subcommand and re-exports
//! `oh_my_ib::ib::{shape_completed_orders, CompletedOrderRow}` (the import below won't
//! resolve until then).
//!
//! NOT frozen (reviewed-by-reading + operator live acceptance, PRD criterion 8): the gateway
//! drain fn — `completed_orders(false)` (api_only hardcoded, ADR 0015), `Orders::OrderData`
//! arm only, End-terminated `iter_data()` (drain-to-End class — NO take-first timeout),
//! `--account` filter-when-set semantics (`orders` parity), `{"completed_orders": …}` wrapper.
//! READ-ONLY red line: the impl diff must contain no place/modify/cancel paths.

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::{json, Value};

use oh_my_ib::ib::{shape_completed_orders, CompletedOrderRow};

fn omi() -> Command {
    Command::cargo_bin("omi").expect("the `omi` binary should build")
}

// ---- fixtures (plain rows; NO ibapi types) ----

fn row(order_id: i32, symbol: &str, status: &str) -> CompletedOrderRow {
    CompletedOrderRow {
        order_id,
        account: "TEST".into(),
        symbol: symbol.into(),
        conid: 265598,
        action: "Buy".into(),
        quantity: 100.0,
        order_type: "LMT".into(),
        limit_price: Some(210.5),
        aux_price: None,
        tif: "Day".into(),
        status: status.into(),
        filled_quantity: 100.0,
        completed_time: "20260703 10:30:00 US/Eastern".into(),
        completed_status: "Filled".into(),
    }
}

// ---- pure shaping seam (gateway order → array of exact 14-key rows) ----

#[test]
fn row_has_exactly_the_fourteen_contract_keys() {
    let out = shape_completed_orders(vec![row(1001, "AAPL", "Filled")]);
    let arr = out.as_array().expect("output must be a JSON array");
    assert_eq!(arr.len(), 1);
    let r = arr[0].as_object().expect("each row must be a JSON object");
    let mut keys: Vec<&str> = r.keys().map(|k| k.as_str()).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "account",
            "action",
            "aux_price",
            "completed_status",
            "completed_time",
            "conid",
            "filled_quantity",
            "limit_price",
            "order_id",
            "order_type",
            "quantity",
            "status",
            "symbol",
            "tif"
        ]
    );
}

#[test]
fn field_values_pass_through_verbatim() {
    let out = shape_completed_orders(vec![row(1001, "AAPL", "Filled")]);
    let r = &out.as_array().unwrap()[0];
    assert_eq!(r["order_id"], json!(1001));
    assert_eq!(r["account"], json!("TEST"));
    assert_eq!(r["symbol"], json!("AAPL"));
    assert_eq!(r["conid"], json!(265598));
    assert_eq!(r["action"], json!("Buy"));
    assert_eq!(r["quantity"], json!(100.0));
    assert_eq!(r["order_type"], json!("LMT"));
    assert_eq!(r["limit_price"], json!(210.5));
    assert_eq!(r["aux_price"], Value::Null, "None aux_price must be null");
    assert_eq!(r["tif"], json!("Day"));
    assert_eq!(r["status"], json!("Filled"));
    assert_eq!(r["filled_quantity"], json!(100.0));
    assert_eq!(r["completed_time"], json!("20260703 10:30:00 US/Eastern"));
    assert_eq!(r["completed_status"], json!("Filled"));
}

#[test]
fn none_prices_are_null_not_absent() {
    let mut r = row(7, "MKT", "Filled");
    r.limit_price = None;
    r.aux_price = None;
    let out = shape_completed_orders(vec![r]);
    let obj = out.as_array().unwrap()[0].as_object().unwrap().clone();
    assert!(obj.contains_key("limit_price"), "null keys must stay present");
    assert_eq!(obj["limit_price"], Value::Null);
    assert_eq!(obj["aux_price"], Value::Null);
}

#[test]
fn empty_completion_strings_pass_through() {
    // Server-version variance: decoder yields "" when the gateway omits completion fields.
    let mut r = row(8, "X", "Cancelled");
    r.completed_time = "".into();
    r.completed_status = "".into();
    let out = shape_completed_orders(vec![r]);
    let row0 = &out.as_array().unwrap()[0];
    assert_eq!(row0["completed_time"], json!(""));
    assert_eq!(row0["completed_status"], json!(""));
    assert_eq!(row0["status"], json!("Cancelled"));
}

#[test]
fn gateway_order_is_preserved_verbatim() {
    let out = shape_completed_orders(vec![row(2, "LATER", "Cancelled"), row(1, "EARLIER", "Filled")]);
    let arr = out.as_array().unwrap();
    assert_eq!(arr[0]["order_id"], json!(2));
    assert_eq!(arr[1]["order_id"], json!(1));
}

#[test]
fn zero_rows_is_an_empty_array() {
    assert_eq!(shape_completed_orders(vec![]), json!([]));
}

// ---- black-box CLI contract ----

#[test]
fn help_lists_completed_orders_subcommand() {
    omi()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("completed-orders"));
}

#[test]
fn completed_orders_help_succeeds() {
    omi().args(["completed-orders", "--help"]).assert().success();
}

#[test]
fn completed_orders_dead_port_is_connection_error_envelope() {
    let output = omi()
        .args([
            "--format", "json", "completed-orders", "--host", "127.0.0.1", "--port", "65000",
        ])
        .assert()
        .failure()
        .get_output()
        .clone();
    let stderr = String::from_utf8_lossy(&output.stderr);
    let v: Value =
        serde_json::from_str(stderr.trim()).expect("stderr must be a JSON error envelope");
    assert_eq!(v["error"]["code"], "connection");
}
