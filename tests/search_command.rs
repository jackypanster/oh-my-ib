//! FROZEN SPEC — search-command (card 01). Offline. The coder must NOT edit this file.
//!
//! Freezes the pure shaping seam `shape_search` (ADR 0014: rows in gateway order → JSON array
//! of exact 7-key rows, full pass-through — no filtering, no re-ranking, "" stays "", empty ⇒
//! `[]`) and the black-box CLI contract of `omi search` (help lists it; missing pattern ⇒
//! usage envelope; dead port ⇒ connection envelope). RED until impl adds the `Search`
//! subcommand and re-exports `oh_my_ib::ib::{shape_search, SearchRow}` (the import below won't
//! resolve, and `--help` won't list `search`, until then).
//!
//! NOT frozen (reviewed-by-reading + operator live acceptance, PRD criterion 8): the gateway
//! fn `search` — the single `matching_symbols` call (plain bounded call class, ADR 0014: no
//! subscription lifecycle, no timeout wrapping, NO STK guard — search is metadata, not
//! market-data), field mapping from `ContractDescription`, `data` error context `search`.

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::{json, Value};

use oh_my_ib::ib::{shape_search, SearchRow};

fn omi() -> Command {
    Command::cargo_bin("omi").expect("the `omi` binary should build")
}

// ---- fixtures (plain rows; NO ibapi types) ----

fn row(conid: i32, symbol: &str, description: &str) -> SearchRow {
    SearchRow {
        conid,
        symbol: symbol.into(),
        sec_type: "STK".into(),
        primary_exchange: "NASDAQ".into(),
        currency: "USD".into(),
        description: description.into(),
        derivative_sec_types: vec!["OPT".into(), "WAR".into()],
    }
}

// ---- pure shaping seam (gateway order → array of exact 7-key rows) ----

#[test]
fn row_passes_through_with_exactly_the_seven_contract_keys() {
    let out = shape_search(vec![row(265598, "AAPL", "APPLE INC")]);
    let arr = out.as_array().expect("search output must be a JSON array");
    assert_eq!(arr.len(), 1);
    let r = arr[0].as_object().expect("each match must be a JSON object");
    let mut keys: Vec<&str> = r.keys().map(|k| k.as_str()).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "conid",
            "currency",
            "derivative_sec_types",
            "description",
            "primary_exchange",
            "sec_type",
            "symbol"
        ]
    );
    assert_eq!(r["conid"], json!(265598));
    assert_eq!(r["symbol"], json!("AAPL"));
    assert_eq!(r["sec_type"], json!("STK"));
    assert_eq!(r["primary_exchange"], json!("NASDAQ"));
    assert_eq!(r["currency"], json!("USD"));
    assert_eq!(r["description"], json!("APPLE INC"));
    assert_eq!(r["derivative_sec_types"], json!(["OPT", "WAR"]));
}

#[test]
fn gateway_order_is_preserved_verbatim() {
    // The result order IS the contract (ADR 0014) — no re-sorting, no re-ranking.
    let out = shape_search(vec![
        row(2, "APLE", "SECOND MATCH"),
        row(1, "AAPL", "FIRST WOULD SORT EARLIER"),
    ]);
    let arr = out.as_array().unwrap();
    assert_eq!(arr[0]["conid"], json!(2));
    assert_eq!(arr[1]["conid"], json!(1));
}

#[test]
fn empty_description_passes_through_as_empty_string() {
    // Older servers omit the company name — decoder yields "". Pass-through, NOT null
    // (the pnl_number sentinel rule is for money f64 only; nothing here is money).
    let out = shape_search(vec![row(1, "X", "")]);
    assert_eq!(out.as_array().unwrap()[0]["description"], json!(""));
}

#[test]
fn empty_derivative_list_is_a_present_empty_array() {
    let mut r = row(1, "X", "D");
    r.derivative_sec_types = vec![];
    let out = shape_search(vec![r]);
    assert_eq!(out.as_array().unwrap()[0]["derivative_sec_types"], json!([]));
}

#[test]
fn non_stk_rows_are_not_filtered() {
    // PRD D3: full pass-through — ETF/index/foreign rows stay, agent filters.
    let mut r = row(11, "SPX", "S&P 500 INDEX");
    r.sec_type = "IND".into();
    r.currency = "USD".into();
    let out = shape_search(vec![r]);
    let arr = out.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["sec_type"], json!("IND"));
}

#[test]
fn zero_matches_is_an_empty_array() {
    assert_eq!(shape_search(vec![]), json!([]));
}

// ---- black-box CLI contract ----

#[test]
fn help_lists_search_subcommand() {
    omi()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("search"));
}

#[test]
fn search_help_succeeds() {
    omi().args(["search", "--help"]).assert().success();
}

#[test]
fn missing_pattern_is_a_usage_error_envelope() {
    let output = omi()
        .args(["--format", "json", "search"])
        .assert()
        .failure()
        .get_output()
        .clone();
    let stderr = String::from_utf8_lossy(&output.stderr);
    let v: Value =
        serde_json::from_str(stderr.trim()).expect("stderr must be a JSON error envelope");
    assert_eq!(v["error"]["code"], "usage");
}

#[test]
fn search_dead_port_is_connection_error_envelope() {
    let output = omi()
        .args([
            "--format", "json", "search", "apple", "--host", "127.0.0.1", "--port", "65000",
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
