//! FROZEN SPEC — stk-orders (card 01, PHASE 2 opener). Offline. The coder must NOT edit this file.
//!
//! Freezes the WRITE-PATH safety contract (ADR 0017): the pure order-building seam
//! `build_stk_order` (exact ibapi `Order`/`Contract` fields — LMT/MKT × buy/sell, TIF=DAY),
//! the pure ack seam `shape_order_ack` (exact 6-key object, MKT ⇒ null limit), the **live
//! double gate** (offline-deterministic: the gate precedes any connection), local argument
//! validation, and the CLI surface. RED until impl adds the Buy/Sell/Cancel subcommands and
//! re-exports `oh_my_ib::ib::{build_stk_order, shape_order_ack}`.
//!
//! NOT frozen (reviewed-by-reading + operator PAPER acceptance, PRD criterion 11): the
//! gateway fns — next_valid_order_id allocation, place_order/cancel_order bounded first-ack
//! loops (ADR 0016 Instant-classified pattern), the UNKNOWN-state timeout envelope, the
//! no-retry rule, and the AGENTS.md/CLAUDE.md red-line amendment (verbatim per arch.md).
//! Review polarity: write calls must exist ONLY in src/ib/trade.rs.

use assert_cmd::Command;
use ibapi::contracts::SecurityType;
use ibapi::orders::{Action, TimeInForce};
use predicates::prelude::*;
use serde_json::{json, Value};

use oh_my_ib::ib::{build_stk_order, shape_order_ack};

fn omi() -> Command {
    let mut cmd = Command::cargo_bin("omi").expect("the `omi` binary should build");
    // The gate reads the environment: start every test from a clean slate.
    cmd.env_remove("OMI_ALLOW_LIVE");
    cmd
}

// ---- pure order-building seam (params → exact ibapi Order/Contract) ----

#[test]
fn limit_buy_builds_exact_lmt_order() {
    let (contract, order) = build_stk_order("AAPL", Action::Buy, 100.0, Some(210.5));
    assert_eq!(contract.symbol.to_string(), "AAPL");
    assert!(matches!(contract.security_type, SecurityType::Stock));
    assert!(matches!(order.action, Action::Buy));
    assert_eq!(order.total_quantity, 100.0);
    assert_eq!(order.order_type, "LMT");
    assert_eq!(order.limit_price, Some(210.5));
    assert!(matches!(order.tif, TimeInForce::Day), "v1 is DAY only");
}

#[test]
fn market_sell_builds_exact_mkt_order() {
    let (contract, order) = build_stk_order("MSFT", Action::Sell, 50.0, None);
    assert_eq!(contract.symbol.to_string(), "MSFT");
    assert!(matches!(order.action, Action::Sell));
    assert_eq!(order.total_quantity, 50.0);
    assert_eq!(order.order_type, "MKT");
    assert_eq!(order.limit_price, None, "MKT must carry no limit price");
    assert!(matches!(order.tif, TimeInForce::Day));
}

// ---- pure ack seam (exact 6-key contract) ----

#[test]
fn ack_has_exactly_the_six_contract_keys() {
    let out = shape_order_ack(1001, "Submitted", "AAPL", "BUY", 100.0, Some(210.5));
    let obj = out.as_object().expect("ack must be a JSON object");
    let mut keys: Vec<&str> = obj.keys().map(|k| k.as_str()).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        ["action", "limit_price", "order_id", "quantity", "status", "symbol"]
    );
    assert_eq!(obj["order_id"], json!(1001));
    assert_eq!(obj["status"], json!("Submitted"));
    assert_eq!(obj["action"], json!("BUY"));
    assert_eq!(obj["quantity"], json!(100.0));
    assert_eq!(obj["limit_price"], json!(210.5));
}

#[test]
fn market_ack_has_null_limit_price() {
    let out = shape_order_ack(7, "PreSubmitted", "MSFT", "SELL", 50.0, None);
    assert_eq!(out["limit_price"], Value::Null, "MKT ack: limit_price null, key present");
}

// ---- the live double gate (offline: the gate check precedes any connection) ----

fn expect_error_code(mut cmd: Command, args: &[&str], code: &str) {
    let output = cmd.args(args).assert().failure().get_output().clone();
    let stderr = String::from_utf8_lossy(&output.stderr);
    let v: Value =
        serde_json::from_str(stderr.trim()).expect("stderr must be a JSON error envelope");
    assert_eq!(v["error"]["code"], code, "args {args:?} must yield code={code}: {stderr}");
}

#[test]
fn live_buy_without_env_is_config_error() {
    expect_error_code(omi(), &["--format", "json", "buy", "AAPL", "1", "--limit", "1", "--live"], "config");
}

#[test]
fn live_sell_without_env_is_config_error() {
    expect_error_code(omi(), &["--format", "json", "sell", "AAPL", "1", "--live"], "config");
}

#[test]
fn live_cancel_without_env_is_config_error() {
    expect_error_code(omi(), &["--format", "json", "cancel", "1001", "--live"], "config");
}

#[test]
fn hand_set_live_port_without_env_is_also_gated() {
    // The effective-port rule: --port 4001 must not bypass the gate.
    expect_error_code(
        omi(),
        &["--format", "json", "buy", "AAPL", "1", "--limit", "1", "--port", "4001"],
        "config",
    );
}

#[test]
fn live_buy_with_env_passes_gate_and_fails_on_dead_gateway() {
    // Gate satisfied ⇒ the command proceeds to connect (dead live port ⇒ connection error).
    let mut cmd = omi();
    cmd.env("OMI_ALLOW_LIVE", "1");
    expect_error_code(
        cmd,
        &["--format", "json", "buy", "AAPL", "1", "--limit", "1", "--live", "--host", "127.0.0.1"],
        "connection",
    );
}

#[test]
fn paper_buy_needs_no_gate_dead_port_is_connection_error() {
    // Paper (default port, here a dead one) is ungated: no config error in the way.
    expect_error_code(
        omi(),
        &["--format", "json", "buy", "AAPL", "1", "--limit", "1", "--host", "127.0.0.1", "--port", "65000"],
        "connection",
    );
}

// ---- local validation (usage errors, before gate and connect) ----

#[test]
fn zero_quantity_is_usage_error() {
    expect_error_code(omi(), &["--format", "json", "buy", "AAPL", "0", "--limit", "1"], "usage");
}

#[test]
fn negative_quantity_is_usage_error() {
    expect_error_code(omi(), &["--format", "json", "sell", "AAPL", "--", "-5"], "usage");
}

#[test]
fn zero_limit_is_usage_error() {
    expect_error_code(omi(), &["--format", "json", "buy", "AAPL", "1", "--limit", "0"], "usage");
}

#[test]
fn missing_args_are_usage_errors() {
    expect_error_code(omi(), &["--format", "json", "buy"], "usage");
    expect_error_code(omi(), &["--format", "json", "cancel"], "usage");
}

// ---- CLI surface ----

#[test]
fn help_lists_the_three_write_verbs() {
    omi()
        .arg("--help")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("buy")
                .and(predicate::str::contains("sell"))
                .and(predicate::str::contains("cancel")),
        );
}

#[test]
fn write_verb_helps_succeed() {
    omi().args(["buy", "--help"]).assert().success();
    omi().args(["sell", "--help"]).assert().success();
    omi().args(["cancel", "--help"]).assert().success();
}
