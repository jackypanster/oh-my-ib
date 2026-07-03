//! FROZEN SPEC — multi-quote (card 01). Offline. The coder must NOT edit this file.
//!
//! Freezes the pure N-shaping seam `shape_quotes` (ADR 0013: 1 row ⇒ the bare object,
//! BYTE-IDENTICAL pass-through — the red line; 2+ ⇒ bare array in input order; 0 ⇒ `[]`
//! defensive) and the variadic CLI contract (zero symbols ⇒ usage envelope; two symbols parse
//! and reach the connect path ⇒ dead-port `code="connection"`; help documents the plural).
//! RED until impl makes `QuoteArgs` variadic and re-exports `oh_my_ib::ib::shape_quotes`
//! (the import below won't resolve until then).
//!
//! NOT frozen (reviewed-by-reading + operator live acceptance, PRD criterion 9): the gateway
//! loop — one connect, ONE `switch_market_data_type`, per-symbol `quote_one` snapshot drains
//! (SnapshotEnd-bounded), `quote/<symbol>` error contexts, sequential consume-then-drop.
//! Existing frozen quote surfaces stay green untouched: `tests/quote_ticks.rs`,
//! `tests/data_commands.rs` (help md-type line + single-symbol dead-port).

use assert_cmd::Command;
use serde_json::{json, Value};

use oh_my_ib::ib::shape_quotes;

fn omi() -> Command {
    Command::cargo_bin("omi").expect("the `omi` binary should build")
}

/// A rich single-symbol quote object — the exact shape `quote_one` emits.
fn quote_obj(symbol: &str, last: f64) -> Value {
    json!({
        "symbol": symbol,
        "delayed": true,
        "ticks": { "DelayedClose": 122.08, "DelayedLast": last },
    })
}

// ---- pure N-shaping seam (ADR 0013 / PRD criteria 1-2) ----

#[test]
fn one_row_returns_the_bare_object_untouched() {
    // The byte-identity red line: N=1 must be EXACTLY the input object — no wrapper,
    // no added/removed/reordered keys.
    let obj = quote_obj("AAPL", 214.29);
    assert_eq!(shape_quotes(vec![obj.clone()]), obj);
}

#[test]
fn multiple_rows_return_a_bare_array_in_input_order() {
    let (a, m, n) = (
        quote_obj("AAPL", 214.29),
        quote_obj("MSFT", 512.5),
        quote_obj("NVDA", 171.3),
    );
    let out = shape_quotes(vec![a.clone(), m.clone(), n.clone()]);
    assert_eq!(out, json!([a, m, n]), "bare array, input order, rows untouched");
}

#[test]
fn two_rows_are_an_array_not_an_object() {
    let out = shape_quotes(vec![quote_obj("AAPL", 1.0), quote_obj("MSFT", 2.0)]);
    let arr = out.as_array().expect("N>=2 must be a JSON array");
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["symbol"], json!("AAPL"));
    assert_eq!(arr[1]["symbol"], json!("MSFT"));
}

#[test]
fn duplicate_rows_pass_through_no_dedupe() {
    // PRD criterion 6: the agent owns its list — duplicates yield duplicate rows.
    let row = quote_obj("AAPL", 214.29);
    let out = shape_quotes(vec![row.clone(), row.clone()]);
    assert_eq!(out, json!([row.clone(), row]));
}

#[test]
fn empty_input_is_empty_array() {
    // Defensive total behavior; unreachable via CLI (clap requires >=1 symbol).
    assert_eq!(shape_quotes(vec![]), json!([]));
}

// ---- variadic CLI contract ----

#[test]
fn zero_symbols_is_a_usage_error_envelope() {
    let output = omi()
        .args(["--format", "json", "quote"])
        .assert()
        .failure()
        .get_output()
        .clone();
    let stderr = String::from_utf8_lossy(&output.stderr);
    let v: Value =
        serde_json::from_str(stderr.trim()).expect("stderr must be a JSON error envelope");
    assert_eq!(
        v["error"]["code"], "usage",
        "a bare `omi quote` must be a usage error, not a crash or success"
    );
}

#[test]
fn two_symbols_parse_and_reach_connect_dead_port_is_connection_envelope() {
    // Proves the variadic parse end-to-end: two positionals accepted, command proceeds to
    // the (dead) gateway and fails with the standard connection envelope.
    let output = omi()
        .args([
            "--format", "json", "quote", "AAPL", "MSFT", "--host", "127.0.0.1", "--port",
            "65000",
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

#[test]
fn quote_help_documents_plural_symbols() {
    let output = omi()
        .args(["quote", "--help"])
        .assert()
        .success()
        .get_output()
        .clone();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("symbol(s)"),
        "quote --help must document the variadic symbols: {stdout}"
    );
}
