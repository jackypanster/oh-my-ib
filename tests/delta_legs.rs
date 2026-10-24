//! Offline spec — `delta-legs` read-only CC/CSP picker. Pure seams only; the gateway
//! flow is reviewed by reading + paper acceptance.

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;

use oh_my_ib::ib::{
    select_delta_legs, shape_delta_legs, validate_delta_legs_params, DeltaLegsParams, LegKind,
    LegRow, DELTA_MAX, DELTA_MIN, SORT_AIM,
};

fn omi() -> Command {
    Command::cargo_bin("omi").expect("the `omi` binary should build")
}

fn params() -> DeltaLegsParams {
    DeltaLegsParams::default()
}

/// A usable leg: bid/ask both positive so mid exists.
fn leg(right: &str, strike: f64, delta: Option<f64>) -> LegRow {
    LegRow {
        strike,
        bid: Some(2.0),
        ask: Some(2.4),
        delta,
        iv: Some(0.3),
        right: right.to_string(),
    }
}

fn selected(rows: &[LegRow]) -> Vec<oh_my_ib::ib::LegCandidate> {
    // spot 200, 30 DTE: call annualized = (2.2/200) × (365/30) ≈ 0.1338.
    select_delta_legs("AAPL", 200.0, "20261218", 30, rows, &params())
}

#[test]
fn band_is_inclusive_at_both_edges() {
    assert!((DELTA_MIN - 0.15).abs() < 1e-12);
    assert!((DELTA_MAX - 0.25).abs() < 1e-12);
    let rows = [
        leg("C", 210.0, Some(0.15)),
        leg("C", 205.0, Some(-0.25)),
        leg("C", 215.0, Some(0.149)),
        leg("C", 220.0, Some(0.251)),
        leg("C", 225.0, Some(0.20)),
    ];
    let strikes: Vec<f64> = selected(&rows).iter().map(|c| c.strike).collect();
    assert!(strikes.contains(&210.0), "0.15 is inside: {strikes:?}");
    assert!(strikes.contains(&205.0), "-0.25 abs is inside: {strikes:?}");
    assert!(strikes.contains(&225.0), "0.20 is inside: {strikes:?}");
    assert!(!strikes.contains(&215.0), "0.149 is outside: {strikes:?}");
    assert!(!strikes.contains(&220.0), "0.251 is outside: {strikes:?}");
    for c in selected(&rows) {
        assert_eq!(c.band, "recommended");
        assert!(c.abs_delta >= DELTA_MIN && c.abs_delta <= DELTA_MAX);
    }
}

#[test]
fn missing_mid_or_delta_is_dropped() {
    let mut no_bid = leg("P", 180.0, Some(-0.20));
    no_bid.bid = None;
    let mut zero_ask = leg("P", 175.0, Some(-0.20));
    zero_ask.ask = Some(0.0);
    let rows = [no_bid, zero_ask, leg("P", 170.0, None)];
    assert!(selected(&rows).is_empty());
}

#[test]
fn kind_filters_the_side() {
    let rows = [leg("C", 210.0, Some(0.20)), leg("P", 180.0, Some(-0.20))];
    let mut calls_only = params();
    calls_only.kind = LegKind::Covered;
    let calls = select_delta_legs("AAPL", 200.0, "20261218", 30, &rows, &calls_only);
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].right, "C");
    assert!(calls[0].assignment_notional.is_none());

    let mut puts_only = params();
    puts_only.kind = LegKind::Csp;
    let puts = select_delta_legs("AAPL", 200.0, "20261218", 30, &rows, &puts_only);
    assert_eq!(puts.len(), 1);
    assert_eq!(puts[0].right, "P");
    // strike × 100 × 1 contract.
    assert_eq!(puts[0].assignment_notional, Some(18000.0));
}

#[test]
fn sort_aims_at_the_target_midpoint() {
    assert!((SORT_AIM - 0.175).abs() < 1e-12);
    // spot 1, 365 DTE ⇒ annualized == mid (mid/spot × 365/365). mid = (bid+ask)/2.
    // 0.175 sits on the aim; 0.19 is near it; 0.30 is far.
    let mk = |strike: f64, bid: f64, ask: f64| LegRow {
        strike,
        bid: Some(bid),
        ask: Some(ask),
        delta: Some(0.20),
        iv: None,
        right: "C".to_string(),
    };
    let rows = [
        mk(120.0, 0.29, 0.31), // annualized 0.30
        mk(110.0, 0.17, 0.18), // annualized 0.175 — on the aim
        mk(115.0, 0.18, 0.20), // annualized 0.19
    ];
    let ranked = select_delta_legs("AAPL", 1.0, "20261218", 365, &rows, &params());
    let strikes: Vec<f64> = ranked.iter().map(|c| c.strike).collect();
    assert_eq!(strikes, vec![110.0, 115.0, 120.0]);
    assert!(ranked[0].in_target_band);
    assert!(ranked[0].above_floor);
    assert!(!ranked[2].in_target_band);
    assert!(ranked[2].above_floor);
}

#[test]
fn missing_annualized_sorts_last() {
    // dte 0 ⇒ annualized is None even though the leg is otherwise usable.
    let rows = [leg("C", 100.0, Some(0.20)), leg("C", 300.0, Some(0.20))];
    let ranked = select_delta_legs("AAPL", 200.0, "20261218", 0, &rows, &params());
    assert!(ranked.iter().all(|c| c.annualized.is_none()));
    let strikes: Vec<f64> = ranked.iter().map(|c| c.strike).collect();
    // Tie on missing yield, then on |δ|: strike ascending breaks it.
    assert_eq!(strikes, vec![100.0, 300.0]);
}

#[test]
fn below_floor_is_flagged_not_dropped() {
    // spot 1000, 365 DTE, mid 2.2 ⇒ annualized 0.0022, under the 10% floor.
    let rows = [leg("C", 1100.0, Some(0.20))];
    let ranked = select_delta_legs("AAPL", 1000.0, "20261218", 365, &rows, &params());
    assert_eq!(ranked.len(), 1);
    assert!(!ranked[0].above_floor);
    assert!(!ranked[0].in_target_band);
}

#[test]
fn validate_params_rejects_bad_flags() {
    let mut bad = params();
    bad.dte_min = 50;
    assert!(validate_delta_legs_params(&bad).is_err());

    let mut bad = params();
    bad.delta_min = 0.0;
    assert!(validate_delta_legs_params(&bad).is_err());

    let mut bad = params();
    bad.delta_max = 1.0;
    assert!(validate_delta_legs_params(&bad).is_err());

    let mut bad = params();
    bad.delta_min = 0.30;
    bad.delta_max = 0.20;
    assert!(validate_delta_legs_params(&bad).is_err());

    let mut bad = params();
    bad.delta_min = f64::NAN;
    assert!(validate_delta_legs_params(&bad).is_err());

    assert!(validate_delta_legs_params(&params()).is_ok());
}

#[test]
fn shape_says_ranking_is_not_a_recommendation() {
    let v = shape_delta_legs(&params(), "AAPL", Some(200.0), None, None, &[], None);
    let disclaimer = v["disclaimer"].as_str().unwrap();
    assert!(
        disclaimer.contains("排序不是建议下单") || disclaimer.contains("not a trade recommendation"),
        "{disclaimer}"
    );
    assert_eq!(v["delta_band"]["inclusive"], true);
    assert_eq!(v["delta_band"]["min"], serde_json::json!(0.15));
    assert_eq!(v["delta_band"]["max"], serde_json::json!(0.25));
    assert_eq!(v["kpi"]["floor"], serde_json::json!(0.10));
    assert_eq!(v["kpi"]["sort_aim"], serde_json::json!(0.175));
    assert_eq!(v["candidates"], v["recommended"]);
    assert_eq!(v["out_of_band"].as_array().unwrap().len(), 0);
    assert!(v["greeks_source"].as_str().unwrap().contains("ModelOption"));

    let degraded = shape_delta_legs(&params(), "AAPL", None, None, None, &[], Some("no_model_greeks"));
    assert_eq!(degraded["degraded"], "no_model_greeks");
}

#[test]
fn help_has_no_confirm_flag() {
    // `--confirm` exists only as a GLOBAL flag (it applies to write verbs). The
    // command's own argument list must not grow one — that would be a send switch.
    let out = omi()
        .args(["delta-legs", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--kind"))
        .stdout(predicate::str::contains("--delta-min"))
        .stdout(predicate::str::contains("--delta-max"))
        .get_output()
        .clone();
    let help = String::from_utf8_lossy(&out.stdout);
    let own_args = help.split("Options:").next().unwrap_or(&help);
    assert!(
        !own_args.contains("--confirm"),
        "delta-legs grew its own --confirm:\n{own_args}"
    );
}

#[test]
fn help_marks_both_commands_read_only() {
    let out = omi().arg("help").assert().success().get_output().clone();
    let v: Value = serde_json::from_slice(&out.stdout).expect("help JSON");
    for name in ["spreads", "delta-legs"] {
        let entry = v["commands"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["name"] == name)
            .unwrap_or_else(|| panic!("{name} missing from help"));
        assert_eq!(entry["gate"], "read-only", "{name}");
    }
}

#[test]
fn dead_port_is_a_connection_error() {
    let out = omi()
        .args(["--format", "json", "delta-legs", "AAPL", "--host", "127.0.0.1", "--port", "65000"])
        .assert()
        .failure()
        .get_output()
        .clone();
    let v: Value = serde_json::from_slice(&out.stderr).expect("stderr JSON");
    assert_eq!(v["error"]["code"], "connection");
}

#[test]
fn bad_flags_are_usage_before_connect() {
    let out = omi()
        .args([
            "--format", "json", "delta-legs", "AAPL", "--delta-min", "1.5", "--host", "127.0.0.1",
            "--port", "65000",
        ])
        .assert()
        .failure()
        .get_output()
        .clone();
    let v: Value = serde_json::from_slice(&out.stderr).expect("stderr JSON");
    assert_eq!(v["error"]["code"], "usage");
    assert_eq!(out.status.code(), Some(64));
}
