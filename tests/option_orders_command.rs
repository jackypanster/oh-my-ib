//! FROZEN SPEC — option-orders (card 01, Phase 2 step 3). Offline. The coder must NOT edit
//! this file.
//!
//! Freezes the single-leg option WRITE surface (ADR 0020): the pure order-building seam
//! `build_option_order` (exact ibapi Contract/Order fields — LMT-ONLY: order_type "LMT",
//! limit always Some, TIF=DAY; the options-read builder chain: SMART/USD, multiplier "100",
//! zero-padded expiry string), the pure ack seam `shape_option_order_ack` (exact 9-key
//! object), the live double-gate parity (offline: gate precedes connection; effective-port
//! rule), local argument validation (usage < config < connection ordering; every numeric
//! finite-checked — qty additionally whole-contract integral >= 1), and the CLI surface.
//! RED until impl adds OptionBuy/OptionSell and re-exports
//! `oh_my_ib::ib::{build_option_order, shape_option_order_ack}`.
//!
//! NOT frozen (reviewed-by-reading + operator PAPER acceptance, PRD criterion 10): the
//! gateway fns (place_core extraction — stk byte-identity guarded by the EXISTING stk
//! frozen suite; ADR 0018 allocation; bounded first-ack; UNKNOWN-state timeout envelope;
//! no-retry), the option_quote.rs pub(crate) visibility promotion, and the docs amendment.
//! DELIBERATE OMISSION: no "env+live+dead-port => connection" gate-pass test here — its
//! stk twin exercises the SHARED require_live_write_gate, and an option twin would place a
//! real LIVE option order if the live gateway happened to be up during a test run.
//! Review polarity: write calls must exist ONLY in src/ib/trade.rs.

use assert_cmd::Command;
use ibapi::contracts::{OptionRight, SecurityType};
use ibapi::orders::{Action, TimeInForce};
use predicates::prelude::*;
use serde_json::{json, Value};

use oh_my_ib::ib::{build_option_order, shape_option_order_ack};

fn omi() -> Command {
    let mut cmd = Command::cargo_bin("omi").expect("the `omi` binary should build");
    // The gate reads the environment: start every test from a clean slate.
    cmd.env_remove("OMI_ALLOW_LIVE");
    cmd
}

fn expect_error_code(mut cmd: Command, args: &[&str], code: &str) {
    let output = cmd.args(args).assert().failure().get_output().clone();
    let stderr = String::from_utf8_lossy(&output.stderr);
    let v: Value =
        serde_json::from_str(stderr.trim()).expect("stderr must be a JSON error envelope");
    assert_eq!(v["error"]["code"], code, "args {args:?} must yield code={code}: {stderr}");
}

// ---- pure order-building seam (params → exact ibapi Contract/Order; LMT-only) ----

#[test]
fn call_buy_builds_exact_lmt_option_order() {
    let (contract, order) = build_option_order(
        "AAPL", (2026, 9, 18), 250.0, OptionRight::Call, None, "SMART", "USD",
        Action::Buy, 2.0, 5.5,
    );
    assert_eq!(contract.symbol.to_string(), "AAPL");
    assert!(matches!(contract.security_type, SecurityType::Option));
    assert_eq!(
        contract.last_trade_date_or_contract_month, "20260918",
        "expiry must serialize zero-padded YYYYMMDD"
    );
    assert_eq!(contract.strike, 250.0);
    assert!(matches!(contract.right, Some(OptionRight::Call)));
    assert_eq!(contract.multiplier, "100", "US equity option builder default");
    assert!(matches!(order.action, Action::Buy));
    assert_eq!(order.total_quantity, 2.0);
    assert_eq!(order.order_type, "LMT", "v1 is LMT-only (ADR 0020)");
    assert_eq!(order.limit_price, Some(5.5));
    assert!(matches!(order.tif, TimeInForce::Day), "v1 is DAY only");
}

#[test]
fn put_sell_with_trading_class_mirrors() {
    let (contract, order) = build_option_order(
        "SPX", (2026, 12, 18), 5000.0, OptionRight::Put, Some("SPXW"), "SMART", "USD",
        Action::Sell, 1.0, 12.0,
    );
    assert!(matches!(contract.right, Some(OptionRight::Put)));
    assert_eq!(contract.last_trade_date_or_contract_month, "20261218");
    assert_eq!(contract.trading_class, "SPXW");
    assert!(matches!(order.action, Action::Sell));
    assert_eq!(order.order_type, "LMT");
    assert_eq!(order.limit_price, Some(12.0));
}

#[test]
fn single_digit_month_and_day_are_zero_padded() {
    let (contract, _) = build_option_order(
        "AAPL", (2027, 1, 2), 300.0, OptionRight::Call, None, "SMART", "USD",
        Action::Buy, 1.0, 1.0,
    );
    assert_eq!(contract.last_trade_date_or_contract_month, "20270102");
}

// ---- pure ack seam (exact 9-key contract) ----

#[test]
fn option_ack_has_exactly_the_nine_contract_keys() {
    let out = shape_option_order_ack(
        1001, "PreSubmitted", "AAPL", "20260918", 250.0, "C", "BUY", 1.0, 5.5,
    );
    let obj = out.as_object().expect("ack must be a JSON object");
    let mut keys: Vec<&str> = obj.keys().map(|k| k.as_str()).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        ["action", "expiry", "limit_price", "order_id", "quantity", "right", "status", "strike", "symbol"]
    );
    assert_eq!(obj["order_id"], json!(1001));
    assert_eq!(obj["status"], json!("PreSubmitted"));
    assert_eq!(obj["symbol"], json!("AAPL"));
    assert_eq!(obj["expiry"], json!("20260918"));
    assert_eq!(obj["strike"], json!(250.0));
    assert_eq!(obj["right"], json!("C"));
    assert_eq!(obj["action"], json!("BUY"));
    assert_eq!(obj["quantity"], json!(1.0));
    assert_eq!(obj["limit_price"], json!(5.5), "LMT-only: always a number, never null");
}

// ---- the live double gate (offline: gate precedes any connection; stk parity) ----

const VALID_BUY: &[&str] = &[
    "--format", "json", "option-buy", "--symbol", "AAPL", "--expiry", "20260918",
    "--strike", "250", "--right", "C", "--qty", "1", "--limit", "0.05",
];

fn with(base: &[&str], extra: &[&str]) -> Vec<String> {
    base.iter().chain(extra.iter()).map(|s| s.to_string()).collect()
}

#[test]
fn live_option_buy_without_env_is_config_error() {
    let args = with(VALID_BUY, &["--live"]);
    let args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    expect_error_code(omi(), &args, "config");
}

#[test]
fn live_option_sell_without_env_is_config_error() {
    expect_error_code(
        omi(),
        &[
            "--format", "json", "option-sell", "--symbol", "AAPL", "--expiry", "20260918",
            "--strike", "250", "--right", "C", "--qty", "1", "--limit", "999", "--live",
        ],
        "config",
    );
}

#[test]
fn hand_set_live_port_without_env_is_also_gated() {
    // Effective-port rule parity: --port 4001 must not bypass the gate.
    let args = with(VALID_BUY, &["--port", "4001"]);
    let args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    expect_error_code(omi(), &args, "config");
}

#[test]
fn paper_dead_port_is_connection_error() {
    // Paper is ungated: valid args + dead port reach the connect step.
    let args = with(VALID_BUY, &["--host", "127.0.0.1", "--port", "65000"]);
    let args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    expect_error_code(omi(), &args, "connection");
}

// ---- local validation (usage; precedes gate AND connect) ----

fn buy_with(overrides: &[(&str, &str)]) -> Vec<String> {
    // VALID_BUY with individual flag values replaced.
    let mut args: Vec<String> = VALID_BUY.iter().map(|s| s.to_string()).collect();
    for (flag, value) in overrides {
        let i = args.iter().position(|a| a == flag).expect("flag present in template");
        args[i + 1] = value.to_string();
    }
    args
}

fn expect_usage(args: Vec<String>) {
    let args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    expect_error_code(omi(), &args, "usage");
}

#[test]
fn zero_qty_is_usage_error() {
    expect_usage(buy_with(&[("--qty", "0")]));
}

#[test]
fn fractional_qty_is_usage_error() {
    expect_usage(buy_with(&[("--qty", "1.5")]));
}

#[test]
fn non_finite_qty_is_usage_error() {
    expect_usage(buy_with(&[("--qty", "inf")]));
}

#[test]
fn zero_limit_is_usage_error() {
    expect_usage(buy_with(&[("--limit", "0")]));
}

#[test]
fn non_finite_limit_is_usage_error() {
    expect_usage(buy_with(&[("--limit", "NaN")]));
}

#[test]
fn non_finite_strike_is_usage_error() {
    expect_usage(buy_with(&[("--strike", "inf")]));
}

#[test]
fn bad_right_is_usage_error() {
    expect_usage(buy_with(&[("--right", "X")]));
}

#[test]
fn dashed_expiry_is_usage_error() {
    expect_usage(buy_with(&[("--expiry", "2026-09-18")]));
}

#[test]
fn validation_precedes_the_live_gate() {
    // usage < config: a bad qty WITH --live and no env must be usage, not config.
    let mut args = buy_with(&[("--qty", "0")]);
    args.push("--live".to_string());
    expect_usage(args);
}

#[test]
fn missing_limit_is_usage_error() {
    // LMT-only: --limit is required (clap-level).
    expect_error_code(
        omi(),
        &[
            "--format", "json", "option-buy", "--symbol", "AAPL", "--expiry", "20260918",
            "--strike", "250", "--right", "C", "--qty", "1",
        ],
        "usage",
    );
}

#[test]
fn missing_args_are_usage_errors() {
    expect_error_code(omi(), &["--format", "json", "option-buy"], "usage");
    expect_error_code(omi(), &["--format", "json", "option-sell"], "usage");
}

// ---- CLI surface ----

#[test]
fn help_lists_the_option_verbs() {
    omi()
        .arg("--help")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("option-buy").and(predicate::str::contains("option-sell")),
        );
}

#[test]
fn option_verb_helps_succeed() {
    omi().args(["option-buy", "--help"]).assert().success();
    omi().args(["option-sell", "--help"]).assert().success();
}
