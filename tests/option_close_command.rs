//! FROZEN SPEC — option-close card 02 (ADR 0022). Offline. The coder must NOT edit this file.
//!
//! Freezes the close-by-conid WRITE surface: the pure derivation seam `derive_close`
//! (the held position's SIGN is the ONLY side authority: long ⇒ SELL, short ⇒ BUY;
//! default = full close; over-close/zero-position/invalid qty ⇒ Err — the anti-double
//! gate), the pure ack seam `shape_option_close_ack` (exact 10-key object), the live
//! double-gate parity (offline: gate precedes connection; effective-port rule), local
//! argument validation (usage < config < connection ordering; conid >= 1; limit finite > 0;
//! qty finite ∧ whole ∧ >= 1), and the CLI surface.
//! RED until impl adds OptionClose and re-exports
//! `oh_my_ib::ib::{derive_close, shape_option_close_ack}`.
//!
//! NOT frozen (reviewed-by-reading + operator PAPER acceptance, PRD criterion 12): the
//! gateway fn `option_close` — single-connect drain→match (not-held/flat ⇒ not_found, the
//! anti-open gate; non-OPT conid ⇒ usage naming the sec_type), rebuild via
//! `build_option_order` VERBATIM, `contract_details` first-row conid assert (the
//! wrong-contract gate, BEFORE place), bounded first-ack, UNKNOWN-state timeout envelope,
//! no-retry — and the docs amendment.
//! DELIBERATE OMISSION: no "env+live+dead-port ⇒ connection" gate-pass test — its stk twin
//! exercises the SHARED require_live_write_gate, and an option twin would place a REAL live
//! order if the live gateway happened to be up during a test run (option-orders precedent).
//! Review polarity: write calls must exist ONLY in src/ib/trade.rs.

use assert_cmd::Command;
use ibapi::orders::Action;
use predicates::prelude::*;
use serde_json::{json, Value};

use oh_my_ib::ib::{derive_close, shape_option_close_ack};

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

// ---- pure derivation seam (position sign → side; the anti-double gate) ----

#[test]
fn long_position_closes_with_sell_full_by_default() {
    assert_eq!(derive_close(2.0, None), Ok((Action::Sell, 2.0)));
}

#[test]
fn short_position_closes_with_buy_full_by_default() {
    assert_eq!(derive_close(-3.0, None), Ok((Action::Buy, 3.0)));
}

#[test]
fn partial_close_keeps_the_derived_side() {
    assert_eq!(derive_close(5.0, Some(2.0)), Ok((Action::Sell, 2.0)));
    assert_eq!(derive_close(-5.0, Some(2.0)), Ok((Action::Buy, 2.0)));
}

#[test]
fn over_close_is_rejected_a_close_never_flips_a_position() {
    assert!(derive_close(2.0, Some(3.0)).is_err());
    assert!(derive_close(-2.0, Some(3.0)).is_err());
}

#[test]
fn zero_position_is_rejected_nothing_to_close() {
    assert!(derive_close(0.0, None).is_err());
    assert!(derive_close(0.0, Some(1.0)).is_err());
}

#[test]
fn invalid_explicit_qty_is_rejected() {
    assert!(derive_close(2.0, Some(0.0)).is_err(), "zero qty");
    assert!(derive_close(2.0, Some(1.5)).is_err(), "fractional contracts");
    assert!(derive_close(2.0, Some(f64::INFINITY)).is_err(), "non-finite");
    assert!(derive_close(2.0, Some(f64::NAN)).is_err(), "NaN");
}

// ---- pure ack seam (exact 10-key contract; identity echoes the MATCHED row) ----

#[test]
fn close_ack_has_exactly_the_ten_contract_keys() {
    let out = shape_option_close_ack(
        1001, "PreSubmitted", 495512569, "AAPL", "20260918", 240.0, "C", "SELL", 2.0, 3.2,
    );
    let obj = out.as_object().expect("ack must be a JSON object");
    let mut keys: Vec<&str> = obj.keys().map(|k| k.as_str()).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "action", "conid", "expiry", "limit_price", "order_id", "quantity", "right",
            "status", "strike", "symbol"
        ]
    );
    assert_eq!(obj["order_id"], json!(1001));
    assert_eq!(obj["status"], json!("PreSubmitted"));
    assert_eq!(obj["conid"], json!(495512569));
    assert_eq!(obj["symbol"], json!("AAPL"));
    assert_eq!(obj["expiry"], json!("20260918"));
    assert_eq!(obj["strike"], json!(240.0));
    assert_eq!(obj["right"], json!("C"));
    assert_eq!(obj["action"], json!("SELL"), "the DERIVED side, never user input");
    assert_eq!(obj["quantity"], json!(2.0));
    assert_eq!(obj["limit_price"], json!(3.2), "LMT-only: always a number, never null");
}

// ---- the live double gate (offline: gate precedes any connection; stk parity) ----

const VALID_CLOSE: &[&str] = &[
    "--format", "json", "option-close", "--conid", "495512569", "--limit", "3.2",
];

fn with(base: &[&str], extra: &[&str]) -> Vec<String> {
    base.iter().chain(extra.iter()).map(|s| s.to_string()).collect()
}

#[test]
fn live_option_close_without_env_is_config_error() {
    let args = with(VALID_CLOSE, &["--live"]);
    let args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    expect_error_code(omi(), &args, "config");
}

#[test]
fn hand_set_live_port_without_env_is_also_gated() {
    // Effective-port rule parity: --port 4001 must not bypass the gate.
    let args = with(VALID_CLOSE, &["--port", "4001"]);
    let args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    expect_error_code(omi(), &args, "config");
}

#[test]
fn paper_dead_port_is_connection_error() {
    // Paper is ungated: valid args + dead port reach the connect step.
    let args = with(VALID_CLOSE, &["--host", "127.0.0.1", "--port", "65000"]);
    let args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    expect_error_code(omi(), &args, "connection");
}

// ---- local validation (usage; precedes gate AND connect) ----

fn close_with(overrides: &[(&str, &str)]) -> Vec<String> {
    // VALID_CLOSE with individual flag values replaced.
    let mut args: Vec<String> = VALID_CLOSE.iter().map(|s| s.to_string()).collect();
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
fn zero_conid_is_usage_error() {
    expect_usage(close_with(&[("--conid", "0")]));
}

#[test]
fn zero_limit_is_usage_error() {
    expect_usage(close_with(&[("--limit", "0")]));
}

#[test]
fn non_finite_limit_is_usage_error() {
    expect_usage(close_with(&[("--limit", "inf")]));
    expect_usage(close_with(&[("--limit", "NaN")]));
}

#[test]
fn zero_qty_is_usage_error() {
    let mut args = close_with(&[]);
    args.extend(["--qty".to_string(), "0".to_string()]);
    expect_usage(args);
}

#[test]
fn fractional_qty_is_usage_error() {
    let mut args = close_with(&[]);
    args.extend(["--qty".to_string(), "1.5".to_string()]);
    expect_usage(args);
}

#[test]
fn non_finite_qty_is_usage_error() {
    let mut args = close_with(&[]);
    args.extend(["--qty".to_string(), "inf".to_string()]);
    expect_usage(args);
}

#[test]
fn validation_precedes_the_live_gate() {
    // usage < config: a bad conid WITH --live and no env must be usage, not config.
    let mut args = close_with(&[("--conid", "0")]);
    args.push("--live".to_string());
    expect_usage(args);
}

#[test]
fn missing_limit_is_usage_error() {
    // LMT-only: --limit is required (clap-level).
    expect_error_code(
        omi(),
        &["--format", "json", "option-close", "--conid", "495512569"],
        "usage",
    );
}

#[test]
fn missing_args_are_usage_errors() {
    expect_error_code(omi(), &["--format", "json", "option-close"], "usage");
}

// ---- CLI surface ----

#[test]
fn help_lists_the_close_verb() {
    omi()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("option-close"));
}

#[test]
fn option_close_help_succeeds() {
    omi().args(["option-close", "--help"]).assert().success();
}
