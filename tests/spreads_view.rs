//! Offline spec — `spreads` income-book view. Pure seam only; the gateway drain is
//! reviewed by reading + paper acceptance.

use assert_cmd::Command;
use serde_json::{json, Value};

use oh_my_ib::ib::{shape_spreads, SpreadsRow};

fn omi() -> Command {
    Command::cargo_bin("omi").expect("the `omi` binary should build")
}

fn row(
    symbol: &str,
    sec_type: &str,
    qty: f64,
    expiry: Option<&str>,
    strike: Option<f64>,
    right: Option<&str>,
    conid: i32,
) -> SpreadsRow {
    SpreadsRow {
        symbol: symbol.to_string(),
        sec_type: sec_type.to_string(),
        qty,
        expiry: expiry.map(str::to_string),
        strike,
        right: right.map(str::to_string),
        conid,
        market_price: 1.5,
        market_value: qty * 1.5,
        avg_cost: 2.0,
        unrealized_pnl: -0.5,
    }
}

fn shape(rows: &[SpreadsRow]) -> Value {
    shape_spreads("DU123", rows)
}

#[test]
fn empty_book_is_empty_lists_and_zero_notional() {
    let v = shape(&[]);
    assert_eq!(v["account"], "DU123");
    assert_eq!(v["covered"], json!([]));
    assert_eq!(v["short_puts"], json!([]));
    assert_eq!(v["assignment_notional"], json!(0.0));
}

#[test]
fn covered_call_carries_share_coverage() {
    let v = shape(&[
        row("aapl", "STK", 300.0, None, None, None, 1),
        row("AAPL", "OPT", -2.0, Some("20261218"), Some(250.0), Some("C"), 11),
        row("AAPL", "OPT", -1.0, Some("20261218"), Some(260.0), Some("C"), 12),
    ]);
    let covered = v["covered"].as_array().unwrap();
    assert_eq!(covered.len(), 2);
    // Sorted by strike: 250 then 260.
    assert_eq!(covered[0]["strike"], json!(250.0));
    assert_eq!(covered[0]["qty"], json!(-2.0));
    assert_eq!(covered[0]["contracts"], json!(2.0));
    assert_eq!(covered[0]["shares_held"], json!(300.0));
    // short_calls sums EVERY short call on the symbol, not just this row.
    assert_eq!(covered[0]["short_calls"], json!(3.0));
    assert_eq!(covered[0]["free_shares"], json!(0.0));
    assert_eq!(covered[1]["strike"], json!(260.0));
    assert_eq!(covered[1]["short_calls"], json!(3.0));
    assert!(v["short_puts"].as_array().unwrap().is_empty());
}

#[test]
fn short_put_reports_assignment_notional() {
    let v = shape(&[row(
        "NVDA",
        "OPT",
        -2.0,
        Some("20261218"),
        Some(170.0),
        Some("P"),
        21,
    )]);
    let puts = v["short_puts"].as_array().unwrap();
    assert_eq!(puts.len(), 1);
    // strike × 100 × |qty| = 170 × 100 × 2.
    assert_eq!(puts[0]["assignment_notional"], json!(34000.0));
    assert_eq!(puts[0]["contracts"], json!(2.0));
    assert_eq!(v["assignment_notional"], json!(34000.0));
    assert!(v["covered"].as_array().unwrap().is_empty());
}

#[test]
fn long_call_and_bare_stock_are_not_rows() {
    let v = shape(&[
        row("AAPL", "STK", 100.0, None, None, None, 1),
        row("AAPL", "OPT", 1.0, Some("20261218"), Some(200.0), Some("C"), 31),
        row("AAPL", "OPT", 1.0, Some("20261218"), Some(180.0), Some("P"), 32),
    ]);
    assert!(v["covered"].as_array().unwrap().is_empty());
    assert!(v["short_puts"].as_array().unwrap().is_empty());
    assert_eq!(v["assignment_notional"], json!(0.0));
}

#[test]
fn top_level_notional_sums_every_short_put() {
    let v = shape(&[
        row("NVDA", "OPT", -1.0, Some("20261218"), Some(170.0), Some("P"), 21),
        row("AAPL", "OPT", -1.0, Some("20261218"), Some(180.0), Some("P"), 22),
        row("AAPL", "OPT", -1.0, Some("20261218"), Some(250.0), Some("C"), 23),
    ]);
    // 170×100 + 180×100; the short call contributes nothing.
    assert_eq!(v["assignment_notional"], json!(35000.0));
    let symbols: Vec<&str> = v["short_puts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["symbol"].as_str().unwrap())
        .collect();
    assert_eq!(symbols, vec!["AAPL", "NVDA"]);
}

#[test]
fn output_carries_the_ac3_keys() {
    let v = shape(&[]);
    for key in [
        "account",
        "covered",
        "short_puts",
        "assignment_notional",
        "close_hints",
        "disclaimer",
    ] {
        assert!(v.get(key).is_some(), "missing key {key}");
    }
    let hints = &v["close_hints"];
    assert!(hints["covered_call"].as_str().unwrap().contains("covered-call close"));
    assert!(hints["short_put"].as_str().unwrap().contains("short-put close"));
    assert!(hints["option_close"].as_str().unwrap().contains("option-close"));
    assert!(v["disclaimer"].as_str().unwrap().contains("read-only"));
}

#[test]
fn dead_port_is_a_connection_error() {
    let out = omi()
        .args(["--format", "json", "spreads", "--host", "127.0.0.1", "--port", "65000"])
        .assert()
        .failure()
        .get_output()
        .clone();
    let v: Value = serde_json::from_slice(&out.stderr).expect("stderr JSON");
    assert_eq!(v["error"]["code"], "connection");
}
