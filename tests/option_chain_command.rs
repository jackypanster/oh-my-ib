//! FROZEN SPEC — options-read (card 01: option-chain). Offline. The coder must NOT edit this file.
//!
//! Freezes the pure chain-shaping seam `shape_option_chain` (ADR 0019: deterministic output —
//! expirations lexicographic-ascending == chronological for YYYYMMDD, strikes ascending, rows
//! ordered by (exchange, trading_class); exact envelope + row keys; zero rows ⇒ `chains: []`
//! success) and the CLI surface. RED until impl adds the `OptionChain` subcommand and re-exports
//! `oh_my_ib::ib::{shape_option_chain, ChainRow}` (the import below won't resolve, and `--help`
//! won't list `option-chain`, until then).
//!
//! NOT frozen (reviewed-by-reading + operator paper acceptance, PRD criterion 8): the
//! `option_chain(cfg, args)` gateway fn — conid resolve via contract_details FIRST row
//! (ADR 0019 D4), the timeout-wrapped End-bounded reqSecDefOptParams drain (ADR 0016
//! Instant-classified pattern; starved window ⇒ exit-6 timeout envelope), the server-side
//! `--exchange` passthrough, and `--format table`. Tiger `:4001` support = journaled
//! observation, never asserted here.

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::{json, Value};

use oh_my_ib::ib::{shape_option_chain, ChainRow};

fn omi() -> Command {
    Command::cargo_bin("omi").expect("the `omi` binary should build")
}

fn expect_error_code(mut cmd: Command, args: &[&str], code: &str) {
    let output = cmd.args(args).assert().failure().get_output().clone();
    let stderr = String::from_utf8_lossy(&output.stderr);
    let v: Value =
        serde_json::from_str(stderr.trim()).expect("stderr must be a JSON error envelope");
    assert_eq!(v["error"]["code"], code, "args {args:?} must yield code={code}: {stderr}");
}

// ---- fixtures (plain rows; NO ibapi types) ----

fn row(exchange: &str, trading_class: &str, expirations: &[&str], strikes: &[f64]) -> ChainRow {
    ChainRow {
        exchange: exchange.to_string(),
        trading_class: trading_class.to_string(),
        multiplier: "100".to_string(),
        expirations: expirations.iter().map(|s| s.to_string()).collect(),
        strikes: strikes.to_vec(),
    }
}

// ---- pure chain-shaping seam (deterministic sort, exact keys) ----

#[test]
fn shape_sorts_expirations_and_strikes_ascending() {
    let out = shape_option_chain(
        "AAPL",
        265598,
        vec![row(
            "SMART",
            "AAPL",
            &["20261218", "20260117", "20260918"],
            &[110.0, 95.0, 100.0],
        )],
    );
    assert_eq!(out["underlying"], json!("AAPL"));
    assert_eq!(out["conid"], json!(265598));
    let chains = out["chains"].as_array().expect("chains must be an array");
    assert_eq!(chains.len(), 1);
    assert_eq!(
        chains[0]["expirations"],
        json!(["20260117", "20260918", "20261218"]),
        "expirations must sort ascending (gateway sets are unordered)"
    );
    assert_eq!(
        chains[0]["strikes"],
        json!([95.0, 100.0, 110.0]),
        "strikes must sort ascending"
    );
}

#[test]
fn shape_orders_rows_by_exchange_then_trading_class() {
    let out = shape_option_chain(
        "SPX",
        2,
        vec![
            row("SMART", "SPXW", &[], &[]),
            row("CBOE", "SPX", &[], &[]),
            row("SMART", "SPX", &[], &[]),
        ],
    );
    let order: Vec<(String, String)> = out["chains"]
        .as_array()
        .expect("chains must be an array")
        .iter()
        .map(|c| {
            (
                c["exchange"].as_str().expect("exchange").to_string(),
                c["trading_class"].as_str().expect("trading_class").to_string(),
            )
        })
        .collect();
    assert_eq!(
        order,
        [
            ("CBOE".to_string(), "SPX".to_string()),
            ("SMART".to_string(), "SPX".to_string()),
            ("SMART".to_string(), "SPXW".to_string()),
        ],
        "rows must order by (exchange, trading_class)"
    );
}

#[test]
fn chain_row_has_exactly_the_five_keys() {
    let out = shape_option_chain("AAPL", 1, vec![row("SMART", "AAPL", &["20260117"], &[95.0])]);
    let obj = out["chains"][0].as_object().expect("chain row must be an object");
    let mut keys: Vec<&str> = obj.keys().map(|k| k.as_str()).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        ["exchange", "expirations", "multiplier", "strikes", "trading_class"]
    );
    assert_eq!(obj["multiplier"], json!("100"), "multiplier is the IB wire STRING");
}

#[test]
fn empty_rows_shape_to_empty_chains_success() {
    let out = shape_option_chain("AAPL", 42, vec![]);
    let obj = out.as_object().expect("envelope must be a JSON object");
    let mut keys: Vec<&str> = obj.keys().map(|k| k.as_str()).collect();
    keys.sort_unstable();
    assert_eq!(keys, ["chains", "conid", "underlying"]);
    assert_eq!(obj["chains"], json!([]), "gateway answered with zero rows ⇒ empty chains, NOT an error");
}

// ---- CLI surface ----

#[test]
fn help_lists_option_chain() {
    omi()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("option-chain"));
}

#[test]
fn option_chain_help_succeeds() {
    omi().args(["option-chain", "--help"]).assert().success();
}

#[test]
fn missing_symbol_is_usage_error() {
    expect_error_code(omi(), &["--format", "json", "option-chain"], "usage");
}

#[test]
fn dead_port_is_connection_error() {
    expect_error_code(
        omi(),
        &[
            "--format", "json", "option-chain", "AAPL", "--host", "127.0.0.1", "--port", "65000",
        ],
        "connection",
    );
}
