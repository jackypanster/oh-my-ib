//! FROZEN SPEC — pcs-scan (card 01). Offline. The coder must NOT edit this file.
//!
//! Freezes the pure seams of arch.md / ADR 0039: params defaults + validation, skip-reason strings,
//! spot choice, expiry choice, coarse/refine strike planning, target-strike interpolation, PCS leg
//! selection + math, and the output shape (every symbol exactly once, candidates sorted by
//! `return_on_risk` desc). Also the CLI surface: usage errors before connect, dead port stays
//! `connection`, the six flags in `--help`, and `pcs-scan` marked `read-only` in `omi help`.
//! RED until impl exports the seams at `oh_my_ib::ib::*`.
//!
//! NOT frozen (review-by-reading + operator live acceptance after OPRA): the gateway flow,
//! `snapshot_batch` concurrency and drop, chain-row choice, and skip-precedence wiring. A live
//! option snapshot needs a real gateway; the no-mock rule forbids a fake IB server.

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::{json, Map, Value};
use time::{Date, Month};

use oh_my_ib::ib::{
    coarse_strikes, estimate_target_strike, pick_expiry, pick_spot, refine_strikes, select_pcs,
    shape_pcs_scan, validate_params, PcsParams, PutRow, SkipReason, PCS_BATCH_LINES,
};

fn omi() -> Command {
    Command::cargo_bin("omi").expect("the `omi` binary should build")
}

fn approx(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

fn put(strike: f64, bid: f64, ask: f64, delta: Option<f64>) -> PutRow {
    PutRow {
        strike,
        bid: Some(bid),
        ask: Some(ask),
        delta,
        iv: Some(0.4),
    }
}

fn ticks(pairs: &[(&str, f64)]) -> Map<String, Value> {
    pairs.iter().map(|(k, v)| (k.to_string(), json!(v))).collect()
}

fn today() -> Date {
    Date::from_calendar_date(2026, Month::September, 26).expect("valid date")
}

// ---- params + constants ----

#[test]
fn batch_cap_and_defaults() {
    assert_eq!(PCS_BATCH_LINES, 50);
    let p = PcsParams::default();
    assert_eq!((p.dte_min, p.dte_max, p.dte_target), (21, 45, 30));
    assert!(approx(p.delta, 0.20));
    assert!(approx(p.width, 5.0));
    assert!(approx(p.min_credit_ratio, 0.25));
}

#[test]
fn validate_params_rules() {
    assert!(validate_params(&PcsParams::default()).is_ok());
    let bad = |f: fn(&mut PcsParams)| {
        let mut p = PcsParams::default();
        f(&mut p);
        validate_params(&p).is_err()
    };
    assert!(bad(|p| p.dte_min = -1));
    assert!(bad(|p| p.dte_min = 50));
    assert!(bad(|p| p.dte_target = 60));
    assert!(bad(|p| p.delta = 0.0));
    assert!(bad(|p| p.delta = 1.0));
    assert!(bad(|p| p.delta = f64::NAN));
    assert!(bad(|p| p.width = 0.0));
    assert!(bad(|p| p.min_credit_ratio = 1.0));
    let ok = PcsParams {
        min_credit_ratio: 0.0,
        ..PcsParams::default()
    };
    assert!(validate_params(&ok).is_ok());
}

#[test]
fn skip_reason_strings() {
    let all = [
        (SkipReason::NoExpiryInWindow, "no_expiry_in_window"),
        (SkipReason::SpotUnavailable, "spot_unavailable"),
        (SkipReason::NoGreeks, "no_greeks"),
        (SkipReason::NoLongLeg, "no_long_leg"),
        (SkipReason::BelowMinCredit, "below_min_credit"),
        (SkipReason::QuoteError, "quote_error"),
    ];
    for (r, s) in all {
        assert_eq!(r.as_str(), s);
    }
}

// ---- spot ----

#[test]
fn pick_spot_precedence() {
    assert_eq!(pick_spot(&ticks(&[("Last", 100.0), ("Bid", 99.0), ("Ask", 101.5)])), Some(100.0));
    assert_eq!(pick_spot(&ticks(&[("DelayedLast", 42.0), ("DelayedClose", 40.0)])), Some(42.0));
    assert_eq!(pick_spot(&ticks(&[("Last", 0.0), ("Bid", 99.0), ("Ask", 101.0)])), Some(100.0));
    assert_eq!(pick_spot(&ticks(&[("DelayedClose", 767.18), ("DelayedHigh", 772.28)])), Some(767.18));
    assert_eq!(pick_spot(&ticks(&[("Bid", 0.0), ("Ask", 10.0)])), None);
    assert_eq!(pick_spot(&Map::new()), None);
}

// ---- expiry ----

#[test]
fn pick_expiry_nearest_target_in_window() {
    let exps: Vec<String> = ["20261002", "20261016", "20261023", "20261030", "20261120", "bad"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    // dte: 6, 20, 27, 34, 55 — window 21..=45, target 30 ⇒ 27 (|3|) beats 34 (|4|)
    assert_eq!(
        pick_expiry(&exps, today(), &PcsParams::default()),
        Some(("20261023".to_string(), 27))
    );
}

#[test]
fn pick_expiry_tie_prefers_earlier_and_none_outside_window() {
    let tie: Vec<String> = ["20261022", "20261030"].iter().map(|s| s.to_string()).collect();
    // dte 26 and 34 — both 4 from target 30 ⇒ earlier
    assert_eq!(
        pick_expiry(&tie, today(), &PcsParams::default()),
        Some(("20261022".to_string(), 26))
    );
    let none: Vec<String> = ["20261002", "20261120"].iter().map(|s| s.to_string()).collect();
    assert_eq!(pick_expiry(&none, today(), &PcsParams::default()), None);
}

// ---- strike planning ----

#[test]
fn coarse_strikes_even_spread_with_both_ends() {
    let strikes: Vec<f64> = (60..=110).map(f64::from).collect();
    // band [65, 100] = 36 strikes; n=8 ⇒ idx = i*5 ⇒ 65,70,…,100
    assert_eq!(
        coarse_strikes(&strikes, 100.0, 8),
        vec![65.0, 70.0, 75.0, 80.0, 85.0, 90.0, 95.0, 100.0]
    );
    let few = vec![66.0, 80.0, 99.0, 120.0];
    assert_eq!(coarse_strikes(&few, 100.0, 8), vec![66.0, 80.0, 99.0]);
}

#[test]
fn estimate_target_strike_interpolates_or_falls_back() {
    let rows = vec![
        put(80.0, 1.0, 1.1, Some(-0.10)),
        put(90.0, 2.0, 2.2, Some(-0.30)),
        put(100.0, 5.0, 5.2, Some(-0.50)),
    ];
    let k = estimate_target_strike(&rows, 0.20).expect("bracketed");
    assert!(approx(k, 85.0), "k*={k}");
    let all_high = vec![put(90.0, 2.0, 2.2, Some(-0.35)), put(95.0, 3.0, 3.2, Some(-0.45))];
    assert!(approx(estimate_target_strike(&all_high, 0.20).expect("nearest"), 90.0));
    let no_delta = vec![put(90.0, 2.0, 2.2, None)];
    assert_eq!(estimate_target_strike(&no_delta, 0.20), None);
}

#[test]
fn refine_strikes_nearest_plus_width_partners_minus_already() {
    let strikes: Vec<f64> = (70..=100).map(f64::from).collect();
    // nearest 3 to 85.4 ⇒ 85, 86, 84; partners (−5) ⇒ 80, 81, 79; minus already {85}
    assert_eq!(
        refine_strikes(&strikes, 85.4, 5.0, 3, &[85.0]),
        vec![79.0, 80.0, 81.0, 84.0, 86.0]
    );
}

// ---- selection + math ----

fn loose() -> PcsParams {
    PcsParams {
        min_credit_ratio: 0.1,
        ..PcsParams::default()
    }
}

#[test]
fn select_pcs_math() {
    let rows = vec![
        put(90.0, 2.0, 2.2, Some(-0.30)),
        put(85.0, 1.0, 1.2, Some(-0.21)),
        put(80.0, 0.5, 0.6, Some(-0.12)),
    ];
    let c = select_pcs("NVDA", 95.0, "20261023", 27, &rows, &loose()).expect("candidate");
    assert_eq!(c.symbol, "NVDA");
    assert!(approx(c.short.strike, 85.0));
    assert!(approx(c.long.strike, 80.0));
    assert!(approx(c.width, 5.0));
    assert!(approx(c.credit_mid, 0.55));
    assert!(approx(c.credit_natural, 0.4));
    assert!(approx(c.max_loss, 445.0));
    assert!(approx(c.return_on_risk, 55.0 / 445.0));
    assert!(approx(c.breakeven, 84.45));
    assert!(approx(c.pop_approx, 0.79));
    assert!(approx(c.spread_pct, (0.2 / 1.1 + 0.1 / 0.55) / 2.0));
}

#[test]
fn select_pcs_skip_reasons() {
    let no_delta = vec![put(85.0, 1.0, 1.2, None), put(80.0, 0.5, 0.6, None)];
    assert_eq!(
        select_pcs("X", 95.0, "20261023", 27, &no_delta, &loose()).err(),
        Some(SkipReason::NoGreeks)
    );
    let no_long = vec![put(85.0, 1.0, 1.2, Some(-0.21))];
    assert_eq!(
        select_pcs("X", 95.0, "20261023", 27, &no_long, &loose()).err(),
        Some(SkipReason::NoLongLeg)
    );
    let thin = vec![put(85.0, 1.0, 1.2, Some(-0.21)), put(80.0, 0.5, 0.6, Some(-0.12))];
    // credit_mid 0.55 < 0.25 × 5 = 1.25
    assert_eq!(
        select_pcs("X", 95.0, "20261023", 27, &thin, &PcsParams::default()).err(),
        Some(SkipReason::BelowMinCredit)
    );
}

#[test]
fn select_pcs_zero_bid_excluded_and_tie_prefers_lower_strike() {
    // 85 has bid 0 ⇒ unusable; among usable: 90 (|0.30−0.20|=0.10) vs 80 (|0.12−0.20|=0.08) ⇒ 80,
    // whose long leg 75 is absent ⇒ no_long_leg
    let rows = vec![
        put(90.0, 2.0, 2.2, Some(-0.30)),
        put(85.0, 0.0, 1.2, Some(-0.21)),
        put(80.0, 0.5, 0.6, Some(-0.12)),
    ];
    assert_eq!(
        select_pcs("X", 95.0, "20261023", 27, &rows, &loose()).err(),
        Some(SkipReason::NoLongLeg)
    );
    // |Δ| 0.25 and 0.15 are equally near 0.20 ⇒ lower strike (80) is the short
    let tie = vec![
        put(85.0, 1.5, 1.6, Some(-0.25)),
        put(80.0, 1.2, 1.3, Some(-0.15)),
        put(75.0, 0.2, 0.3, Some(-0.08)),
    ];
    let c = select_pcs("X", 95.0, "20261023", 27, &tie, &loose()).expect("candidate");
    assert!(approx(c.short.strike, 80.0));
    assert!(approx(c.long.strike, 75.0));
}

// ---- output shape ----

#[test]
fn shape_sorts_candidates_and_lists_every_symbol_once() {
    let p = loose();
    let lo = select_pcs(
        "AAA",
        95.0,
        "20261023",
        27,
        &[put(85.0, 1.0, 1.2, Some(-0.21)), put(80.0, 0.5, 0.6, Some(-0.12))],
        &p,
    )
    .expect("lo");
    let hi = select_pcs(
        "BBB",
        95.0,
        "20261023",
        27,
        &[put(85.0, 2.0, 2.2, Some(-0.21)), put(80.0, 0.5, 0.6, Some(-0.12))],
        &p,
    )
    .expect("hi");
    let v = shape_pcs_scan(
        &p,
        vec![lo, hi],
        vec![
            ("ZZZ".to_string(), SkipReason::NoGreeks),
            ("CCC".to_string(), SkipReason::SpotUnavailable),
        ],
    );
    let cands = v["candidates"].as_array().expect("candidates");
    assert_eq!(cands[0]["symbol"], "BBB", "higher return_on_risk first");
    assert_eq!(cands[1]["symbol"], "AAA");
    for key in [
        "symbol", "spot", "expiry", "dte", "short", "long", "width", "credit_mid", "credit_natural",
        "max_loss", "return_on_risk", "breakeven", "pop_approx", "spread_pct",
    ] {
        assert!(cands[0].get(key).is_some(), "candidate missing `{key}`");
    }
    for key in ["strike", "bid", "ask", "mid", "delta", "iv"] {
        assert!(cands[0]["short"].get(key).is_some(), "short leg missing `{key}`");
    }
    assert_eq!(
        v["skipped"],
        json!([
            {"symbol": "ZZZ", "reason": "no_greeks"},
            {"symbol": "CCC", "reason": "spot_unavailable"}
        ])
    );
    assert_eq!(v["params"]["width"], json!(5.0));
    assert_eq!(v["params"]["dte_target"], json!(30));
}

// ---- CLI surface ----

fn stderr_json(args: &[&str]) -> (i32, Value) {
    let out = omi().args(args).assert().failure().get_output().clone();
    let v: Value = serde_json::from_str(String::from_utf8_lossy(&out.stderr).trim())
        .expect("stderr must be a JSON error envelope");
    (out.status.code().unwrap_or(-1), v)
}

#[test]
fn usage_errors_precede_connect() {
    for bad in [
        vec!["--format", "json", "pcs-scan", "NVDA", "--delta", "1.5"],
        vec!["--format", "json", "pcs-scan", "NVDA", "--dte-min", "50"],
        vec!["--format", "json", "pcs-scan", "NVDA", "--width", "0"],
        vec!["--format", "json", "pcs-scan"],
    ] {
        let mut args = bad.clone();
        args.extend(["--host", "127.0.0.1", "--port", "65000"]);
        let (code, v) = stderr_json(&args);
        assert_eq!(v["error"]["code"], "usage", "args {bad:?}");
        assert_eq!(code, 64, "args {bad:?}");
    }
}

#[test]
fn dead_port_is_a_connection_error() {
    let (_, v) = stderr_json(&[
        "--format", "json", "pcs-scan", "NVDA", "SPY", "--host", "127.0.0.1", "--port", "65000",
    ]);
    assert_eq!(v["error"]["code"], "connection");
}

#[test]
fn help_shows_the_six_flags_and_read_only_gate() {
    let mut a = omi();
    let mut assert = a.args(["pcs-scan", "--help"]).assert().success();
    for flag in [
        "--dte-min",
        "--dte-max",
        "--dte-target",
        "--delta",
        "--width",
        "--min-credit-ratio",
    ] {
        assert = assert.stdout(predicate::str::contains(flag));
    }
    let out = omi().arg("help").assert().success().get_output().clone();
    let v: Value = serde_json::from_slice(&out.stdout).expect("help JSON");
    let entry = v["commands"]
        .as_array()
        .expect("commands")
        .iter()
        .find(|e| e["name"] == "pcs-scan")
        .expect("pcs-scan in help")
        .clone();
    assert_eq!(entry["gate"], "read-only");
}
