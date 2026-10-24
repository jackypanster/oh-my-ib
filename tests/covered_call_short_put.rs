//! `covered-call` / `short-put` open|close.
//!
//! The guards are pure and offline: a refusal is decided from the position rows
//! alone, so nothing is ever sent. The CLI assertions are black-box against the
//! `omi` binary and a dead port.

use assert_cmd::Command;
use predicates::prelude::*;

use oh_my_ib::ib::{
    assignment_notional, check_coverage, check_short_option_held, GuardRow,
};

fn row(
    symbol: &str,
    sec_type: &str,
    qty: f64,
    expiry: Option<&str>,
    strike: Option<f64>,
    right: Option<&str>,
) -> GuardRow {
    GuardRow {
        symbol: symbol.into(),
        sec_type: sec_type.into(),
        qty,
        expiry: expiry.map(str::to_string),
        strike,
        right: right.map(str::to_string),
    }
}

fn stk(symbol: &str, qty: f64) -> GuardRow {
    row(symbol, "STK", qty, None, None, None)
}

fn opt(symbol: &str, qty: f64, expiry: &str, strike: f64, right: &str) -> GuardRow {
    row(symbol, "OPT", qty, Some(expiry), Some(strike), Some(right))
}

// ---- coverage guard -------------------------------------------------------------

#[test]
fn coverage_counts_free_shares_after_existing_short_calls() {
    let rows = vec![stk("AAPL", 300.0), opt("AAPL", -1.0, "20261218", 250.0, "C")];
    let coverage = check_coverage("AAPL", 1.0, &rows).expect("200 free covers 1");
    assert_eq!(coverage.shares, 300.0);
    assert_eq!(coverage.short_calls, 1.0);
}

#[test]
fn coverage_refuses_when_short_calls_already_commit_the_shares() {
    let rows = vec![stk("AAPL", 100.0), opt("AAPL", -1.0, "20261218", 250.0, "C")];
    let err = check_coverage("AAPL", 1.0, &rows).expect_err("0 free");
    assert!(err.contains("AAPL"), "{err}");
    assert!(err.contains("100"), "{err}");
}

#[test]
fn coverage_ignores_puts_and_other_underlyings() {
    let rows = vec![
        stk("AAPL", 100.0),
        opt("AAPL", -5.0, "20261218", 200.0, "P"),
        stk("NVDA", 1000.0),
        opt("NVDA", -3.0, "20261218", 150.0, "C"),
    ];
    let coverage = check_coverage("aapl", 1.0, &rows).expect("puts are not a commitment");
    assert_eq!(coverage.shares, 100.0);
    assert_eq!(coverage.short_calls, 0.0);
}

#[test]
fn coverage_refuses_with_no_position_at_all() {
    assert!(check_coverage("AAPL", 1.0, &[]).is_err());
}

#[test]
fn coverage_refuses_short_stock_and_accepts_fractional_cover() {
    let short_stock = vec![stk("AAPL", -500.0)];
    assert!(check_coverage("AAPL", 1.0, &short_stock).is_err(), "short stock covers nothing");

    let fractional = vec![stk("AAPL", 102.9)];
    assert!(check_coverage("AAPL", 1.0, &fractional).is_ok(), "102.9 covers one contract");
    assert!(check_coverage("AAPL", 2.0, &fractional).is_err(), "102.9 does not cover two");
}

#[test]
fn coverage_boundary_is_exact() {
    assert!(check_coverage("AAPL", 1.0, &[stk("AAPL", 100.0)]).is_ok());
    assert!(check_coverage("AAPL", 1.0, &[stk("AAPL", 99.9)]).is_err());
}

// ---- held-short close guard -------------------------------------------------------

#[test]
fn close_guard_accepts_a_held_short_of_the_exact_contract() {
    let rows = vec![opt("AAPL", -2.0, "20261218", 250.0, "C")];
    let held = check_short_option_held("AAPL", "20261218", 250.0, "C", 1.0, &rows).expect("held");
    assert_eq!(held, -2.0);
}

#[test]
fn close_guard_refuses_a_long_holding_and_a_different_contract() {
    let long = vec![opt("AAPL", 2.0, "20261218", 250.0, "C")];
    assert!(check_short_option_held("AAPL", "20261218", 250.0, "C", 1.0, &long).is_err());

    let other_strike = vec![opt("AAPL", -2.0, "20261218", 260.0, "C")];
    assert!(check_short_option_held("AAPL", "20261218", 250.0, "C", 1.0, &other_strike).is_err());

    let other_right = vec![opt("AAPL", -2.0, "20261218", 250.0, "P")];
    assert!(check_short_option_held("AAPL", "20261218", 250.0, "C", 1.0, &other_right).is_err());
}

#[test]
fn close_guard_refuses_when_qty_exceeds_the_short() {
    let rows = vec![opt("AAPL", -1.0, "20261218", 250.0, "P")];
    assert!(check_short_option_held("AAPL", "20261218", 250.0, "P", 2.0, &rows).is_err());
    assert!(check_short_option_held("AAPL", "20261218", 250.0, "P", 1.0, &rows).is_ok());
}

#[test]
fn close_guard_sums_split_rows_of_the_same_contract() {
    let rows = vec![
        opt("NVDA", -1.0, "20261218", 170.0, "P"),
        opt("NVDA", -2.0, "20261218", 170.0, "P"),
    ];
    let held = check_short_option_held("nvda", "20261218", 170.0, "P", 3.0, &rows).expect("summed");
    assert_eq!(held, -3.0);
}

// ---- assignment notional ----------------------------------------------------------

#[test]
fn assignment_notional_is_strike_times_100_times_qty() {
    assert_eq!(assignment_notional(170.0, 1.0), 17_000.0);
    assert_eq!(assignment_notional(5.0, 2.0), 1_000.0);
}

// ---- CLI surface ------------------------------------------------------------------

fn omi() -> Command {
    Command::cargo_bin("omi").expect("the `omi` binary should build")
}

fn stderr_code(args: &[&str]) -> String {
    let output = omi().args(args).assert().failure().get_output().clone();
    let stderr = String::from_utf8_lossy(&output.stderr);
    let v: serde_json::Value = serde_json::from_str(stderr.trim()).expect("stderr JSON");
    v["error"]["code"].as_str().expect("code").to_string()
}

fn order_flags(qty: &str) -> Vec<&str> {
    vec![
        "--symbol", "AAPL", "--expiry", "20261218", "--strike", "250", "--qty", qty, "--limit",
        "3.5",
    ]
}

#[test]
fn covered_call_open_help_names_the_actions_and_has_no_right_flag() {
    omi()
        .args(["covered-call", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("open"))
        .stdout(predicate::str::contains("close"))
        .stdout(predicate::str::contains("--confirm"));
    omi()
        .args(["covered-call", "open", "--help"])
        .assert()
        .success()
        .stdout(predicates::str::contains("--symbol").and(predicate::str::contains("--right").not()));
}

#[test]
fn missing_action_is_usage() {
    assert_eq!(stderr_code(&["--format", "json", "covered-call"]), "usage");
    assert_eq!(stderr_code(&["--format", "json", "short-put"]), "usage");
}

#[test]
fn bad_qty_is_usage_before_any_connection() {
    let mut args = vec!["--format", "json", "covered-call", "open"];
    args.extend(order_flags("0"));
    assert_eq!(stderr_code(&args), "usage");
}

#[test]
fn live_without_env_is_config_even_on_the_preview_arm() {
    for (verb, action) in [("covered-call", "open"), ("short-put", "close")] {
        let mut args = vec!["--format", "json", "--live", verb, action];
        args.extend(order_flags("1"));
        assert_eq!(stderr_code(&args), "config", "{verb} {action}");
    }
}

#[test]
fn paper_preview_on_a_dead_port_is_a_connection_error() {
    // Past usage and past the live gate: the preview still connects to read
    // positions for the guard, so a dead port is `connection`, never a send.
    let mut args = vec!["--format", "json", "short-put", "open"];
    args.extend(order_flags("1"));
    args.extend(["--host", "127.0.0.1", "--port", "65000"]);
    assert_eq!(stderr_code(&args), "connection");
}
