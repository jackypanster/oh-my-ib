//! FROZEN SPEC — options-read (card 02: option-quote). Offline. The coder must NOT edit this file.
//!
//! Freezes the pure greeks seam `option_quote_greeks` (ADR 0019 D3: ONLY the model computation
//! rows — `ModelOption`/`DelayedModelOption` — yield greeks; bid/ask/last/custom rows and
//! non-computation ticks are ignored), the pure assembly seam `shape_option_quote` (exact 8-key
//! contract echo, right normalized "C"/"P", `greeks` key present IFF a model row arrived,
//! None-valued greeks fields omitted, ticks pass-through), local argument validation
//! (usage errors precede any connection), and the CLI surface. RED until impl adds the
//! `OptionQuote` subcommand and re-exports
//! `oh_my_ib::ib::{option_quote_greeks, shape_option_quote, GreeksRow}`.
//!
//! NOT frozen (reviewed-by-reading + operator paper acceptance, PRD criterion 8): the
//! `option_quote(cfg, args)` gateway fn — OptionBuilder contract construction, md-type switch,
//! the bare SnapshotEnd-bounded snapshot drain (quote.rs class, deliberately NOT timeout-wrapped
//! — ADR 0019 D2), last-model-row-wins accumulation, and `--format table`. Greeks PRESENCE from
//! a live gateway is NEVER asserted (best-effort, PRD D3).

use assert_cmd::Command;
use ibapi::contracts::tick_types::TickType;
use ibapi::contracts::OptionComputation;
use ibapi::market_data::realtime::{TickAttribute, TickPrice};
use ibapi::prelude::TickTypes;
use predicates::prelude::*;
use serde_json::{json, Map, Value};

use oh_my_ib::ib::{option_quote_greeks, shape_option_quote, GreeksRow};

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

// ---- fixtures (OptionComputation has pub fields + Default — quote_ticks.rs precedent) ----

fn computation(field: TickType) -> TickTypes {
    TickTypes::OptionComputation(OptionComputation {
        field,
        implied_volatility: Some(0.31),
        delta: Some(0.52),
        gamma: Some(0.01),
        vega: Some(0.4),
        theta: Some(-0.09),
        option_price: Some(12.4),
        underlying_price: Some(251.2),
        ..Default::default()
    })
}

// ---- pure greeks seam (model rows only — ADR 0019 D3) ----

#[test]
fn model_computation_yields_greeks() {
    let g = option_quote_greeks(&computation(TickType::ModelOption))
        .expect("the model computation row must yield greeks");
    assert_eq!(g.implied_volatility, Some(0.31));
    assert_eq!(g.delta, Some(0.52));
    assert_eq!(g.gamma, Some(0.01));
    assert_eq!(g.vega, Some(0.4));
    assert_eq!(g.theta, Some(-0.09));
    assert_eq!(g.option_price, Some(12.4));
    assert_eq!(g.underlying_price, Some(251.2));
}

#[test]
fn delayed_model_computation_also_yields_greeks() {
    assert!(
        option_quote_greeks(&computation(TickType::DelayedModelOption)).is_some(),
        "delayed model row (tick 83) is A model row — the default md-type is delayed"
    );
}

#[test]
fn side_and_custom_computations_are_ignored() {
    for field in [
        TickType::BidOption,
        TickType::AskOption,
        TickType::LastOption,
        TickType::CustOptionComputation,
    ] {
        // TickType is not Copy: render the label BEFORE `field` moves into computation().
        let label = format!("{field:?}");
        assert!(
            option_quote_greeks(&computation(field)).is_none(),
            "{label} is a per-side computation, not the model greeks row"
        );
    }
}

#[test]
fn non_computation_ticks_are_ignored() {
    let tick = TickTypes::Price(TickPrice {
        tick_type: TickType::DelayedBid,
        price: 248.1,
        attributes: TickAttribute::default(),
    });
    assert!(option_quote_greeks(&tick).is_none(), "price ticks belong to `ticks`, not `greeks`");
}

// ---- pure assembly seam (8-key echo, right normalization, greeks-iff, omit-None) ----

#[test]
fn shape_echoes_the_exact_contract_and_normalizes_right() {
    let out = shape_option_quote(
        "AAPL", "20260918", 250.0, "c", "SMART", "USD", None, true,
        Map::new(),
        None,
    );
    let c = out["contract"].as_object().expect("contract echo must be an object");
    let mut keys: Vec<&str> = c.keys().map(|k| k.as_str()).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        ["currency", "exchange", "expiry", "multiplier", "right", "strike", "symbol", "trading_class"]
    );
    assert_eq!(c["right"], json!("C"), "right must normalize case-insensitively to C|P");
    assert_eq!(c["strike"], json!(250.0));
    assert_eq!(c["multiplier"], json!("100"), "US equity option builder default");
    assert_eq!(c["trading_class"], Value::Null, "absent trading class ⇒ null, key present");
    assert_eq!(out["delayed"], json!(true));
    assert!(
        out.get("greeks").is_none(),
        "no model row arrived ⇒ NO greeks key (best-effort contract, ADR 0019 D3)"
    );
}

#[test]
fn shape_normalizes_put_word_and_echoes_trading_class() {
    let out = shape_option_quote(
        "SPX", "20260917", 5000.0, "put", "SMART", "USD", Some("SPXW"), false,
        Map::new(),
        None,
    );
    assert_eq!(out["contract"]["right"], json!("P"));
    assert_eq!(out["contract"]["trading_class"], json!("SPXW"));
    assert_eq!(out["delayed"], json!(false));
}

#[test]
fn shape_emits_greeks_iff_present_and_omits_none_fields() {
    let mut ticks = Map::new();
    ticks.insert("DelayedBid".to_string(), json!(248.1));
    let greeks = GreeksRow {
        delta: Some(0.5),
        ..Default::default()
    };
    let out = shape_option_quote(
        "AAPL", "20260918", 250.0, "C", "SMART", "USD", None, true,
        ticks,
        Some(greeks),
    );
    assert_eq!(out["ticks"]["DelayedBid"], json!(248.1), "price ticks pass through");
    let g = out["greeks"].as_object().expect("model row arrived ⇒ greeks key present");
    let keys: Vec<&str> = g.keys().map(|k| k.as_str()).collect();
    assert_eq!(keys, ["delta"], "None-valued greeks fields must be OMITTED, not null");
    assert_eq!(g["delta"], json!(0.5));
}

// ---- local validation (usage errors, before any connection) ----

#[test]
fn bad_right_is_usage_error() {
    expect_error_code(
        omi(),
        &["--format", "json", "option-quote", "--symbol", "AAPL", "--expiry", "20260918", "--strike", "250", "--right", "X"],
        "usage",
    );
}

#[test]
fn zero_strike_is_usage_error() {
    expect_error_code(
        omi(),
        &["--format", "json", "option-quote", "--symbol", "AAPL", "--expiry", "20260918", "--strike", "0", "--right", "C"],
        "usage",
    );
}

#[test]
fn negative_strike_is_usage_error() {
    expect_error_code(
        omi(),
        &["--format", "json", "option-quote", "--symbol", "AAPL", "--expiry", "20260918", "--strike=-5", "--right", "C"],
        "usage",
    );
}

#[test]
fn short_expiry_is_usage_error() {
    expect_error_code(
        omi(),
        &["--format", "json", "option-quote", "--symbol", "AAPL", "--expiry", "2026091", "--strike", "250", "--right", "C"],
        "usage",
    );
}

#[test]
fn dashed_expiry_is_usage_error() {
    expect_error_code(
        omi(),
        &["--format", "json", "option-quote", "--symbol", "AAPL", "--expiry", "2026-09-18", "--strike", "250", "--right", "C"],
        "usage",
    );
}

#[test]
fn out_of_range_expiry_is_usage_error() {
    expect_error_code(
        omi(),
        &["--format", "json", "option-quote", "--symbol", "AAPL", "--expiry", "20261332", "--strike", "250", "--right", "C"],
        "usage",
    );
}

#[test]
fn missing_flags_are_usage_errors() {
    expect_error_code(omi(), &["--format", "json", "option-quote"], "usage");
}

// ---- CLI surface + gateway boundary ----

#[test]
fn help_lists_option_quote() {
    omi()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("option-quote"));
}

#[test]
fn option_quote_help_succeeds() {
    omi().args(["option-quote", "--help"]).assert().success();
}

#[test]
fn dead_port_is_connection_error() {
    expect_error_code(
        omi(),
        &[
            "--format", "json", "option-quote", "--symbol", "AAPL", "--expiry", "20260918",
            "--strike", "250", "--right", "C", "--host", "127.0.0.1", "--port", "65000",
        ],
        "connection",
    );
}
