//! FROZEN SPEC — option-combo (card 01, Phase 2 step 4: multi-leg BAG writes). Offline.
//! The coder must NOT edit this file.
//!
//! Freezes the combo WRITE surface (ADR 0021): the pure leg-DSL seam `parse_combo_leg`
//! ("ACTION RATIO SYMBOL EXPIRY STRIKE RIGHT" → LegSpec; normalization + every malformed
//! token class), the pure order-building seam `build_combo_order` (SecurityType::Spread,
//! exact ComboLeg fields in input order, underlying symbol back-fill, LMT with SIGN-FREE
//! net limit — negative = credit — TIF=DAY), the pure ack seam `shape_combo_order_ack`
//! (7 top-level keys + 7-key leg echoes), gate parity (offline), local validation
//! (usage < config < connection; leg errors name the 1-based index), and the CLI surface.
//! RED until impl adds OptionCombo and re-exports
//! `oh_my_ib::ib::{parse_combo_leg, build_combo_order, shape_combo_order_ack, LegSpec}`.
//!
//! NOT frozen (reviewed-by-reading + operator PAPER acceptance, PRD criteria 1-2): the
//! gateway fn (per-leg contract_details conid resolution fail-fast, place_core reuse,
//! bounded first-ack, no-retry), and the docs amendment (two-text rule, CLAUDE.md < 900B).
//! DELIBERATE OMISSION (option-orders precedent): no "env+live+dead ⇒ connection"
//! gate-pass test — the stk twin covers the shared gate; a combo twin would place a real
//! LIVE order if the live gateway were up during a test run.
//! Review polarity: write calls ONLY in src/ib/trade.rs (contract_details is a read call).

use assert_cmd::Command;
use ibapi::contracts::{LegAction, SecurityType};
use ibapi::orders::{Action, TimeInForce};
use predicates::prelude::*;
use serde_json::{json, Value};

use oh_my_ib::ib::{build_combo_order, parse_combo_leg, shape_combo_order_ack, LegSpec};

fn omi() -> Command {
    let mut cmd = Command::cargo_bin("omi").expect("the `omi` binary should build");
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

// ---- pure leg-DSL seam ----

#[test]
fn parse_leg_happy_call_normalizes() {
    let leg = parse_combo_leg("buy 1 aapl 20260918 240 c").expect("valid leg must parse");
    assert_eq!(
        leg,
        LegSpec {
            action: "BUY".to_string(),
            ratio: 1,
            symbol: "AAPL".to_string(),
            expiry: "20260918".to_string(),
            strike: 240.0,
            right: "C".to_string(),
        }
    );
}

#[test]
fn parse_leg_happy_put_with_ratio() {
    let leg = parse_combo_leg("SELL 2 SPX 20261218 5000 PUT").expect("valid leg must parse");
    assert_eq!(leg.action, "SELL");
    assert_eq!(leg.ratio, 2);
    assert_eq!(leg.right, "P");
}

#[test]
fn parse_leg_rejects_every_malformed_token_class() {
    for bad in [
        "BUY 1 AAPL 20260918 240",          // 5 tokens
        "BUY 1 AAPL 20260918 240 C EXTRA",  // 7 tokens
        "HOLD 1 AAPL 20260918 240 C",       // bad action
        "BUY 0 AAPL 20260918 240 C",        // ratio 0
        "BUY 1.5 AAPL 20260918 240 C",      // fractional ratio
        "BUY 1 AAPL 2026-09-18 240 C",      // dashed expiry
        "BUY 1 AAPL 20261332 240 C",        // month 13
        "BUY 1 AAPL 20260918 0 C",          // strike 0
        "BUY 1 AAPL 20260918 inf C",        // non-finite strike
        "BUY 1 AAPL 20260918 240 X",        // bad right
    ] {
        assert!(parse_combo_leg(bad).is_err(), "must reject: {bad}");
    }
}

// ---- pure order-building seam (Spread contract + sign-free net LMT) ----

fn leg(action: &str, ratio: i32, symbol: &str, expiry: &str, strike: f64, right: &str) -> LegSpec {
    LegSpec {
        action: action.to_string(),
        ratio,
        symbol: symbol.to_string(),
        expiry: expiry.to_string(),
        strike,
        right: right.to_string(),
    }
}

#[test]
fn vertical_debit_builds_exact_bag() {
    let long = leg("BUY", 1, "AAPL", "20260918", 240.0, "C");
    let short = leg("SELL", 1, "AAPL", "20260918", 250.0, "C");
    let (contract, order) = build_combo_order(
        "AAPL", &[(&long, 111), (&short, 222)], Action::Buy, 1.0, 2.5, "SMART", "USD",
    )
    .expect("two-leg vertical must build");
    assert!(matches!(contract.security_type, SecurityType::Spread));
    assert_eq!(contract.symbol.to_string(), "AAPL", "underlying symbol must be back-filled");
    assert_eq!(contract.combo_legs.len(), 2);
    assert_eq!(contract.combo_legs[0].contract_id, 111);
    assert_eq!(contract.combo_legs[0].ratio, 1);
    assert!(matches!(contract.combo_legs[0].action, LegAction::Buy));
    assert_eq!(contract.combo_legs[1].contract_id, 222);
    assert!(matches!(contract.combo_legs[1].action, LegAction::Sell));
    assert!(matches!(order.action, Action::Buy));
    assert_eq!(order.total_quantity, 1.0);
    assert_eq!(order.order_type, "LMT", "combos are LMT-only (ADR 0021)");
    assert_eq!(order.limit_price, Some(2.5));
    assert!(matches!(order.tif, TimeInForce::Day));
}

#[test]
fn negative_net_limit_is_a_credit_and_builds() {
    let short = leg("SELL", 1, "SPX", "20261218", 5000.0, "P");
    let long = leg("BUY", 1, "SPX", "20261218", 4900.0, "P");
    let (_, order) = build_combo_order(
        "SPX", &[(&short, 1), (&long, 2)], Action::Buy, 1.0, -0.5, "SMART", "USD",
    )
    .expect("credit spread must build");
    assert_eq!(
        order.limit_price,
        Some(-0.5),
        "NET limit is sign-free: negative = credit (deliberately unlike single-leg)"
    );
}

#[test]
fn leg_ratio_passes_through() {
    let a = leg("BUY", 1, "AAPL", "20260918", 240.0, "C");
    let b = leg("SELL", 2, "AAPL", "20260918", 250.0, "C");
    let (contract, _) = build_combo_order(
        "AAPL", &[(&a, 10), (&b, 20)], Action::Sell, 3.0, 0.0, "SMART", "USD",
    )
    .expect("ratio spread must build");
    assert_eq!(contract.combo_legs[1].ratio, 2);
}

// ---- pure ack seam (7 top-level keys + 7-key leg echoes) ----

#[test]
fn combo_ack_has_exact_shape() {
    let l1 = leg("BUY", 1, "AAPL", "20260918", 240.0, "C");
    let l2 = leg("SELL", 1, "AAPL", "20260918", 250.0, "C");
    let out = shape_combo_order_ack(
        77, "PreSubmitted", "AAPL", "BUY", 1.0, 2.5, &[(&l1, 111), (&l2, 222)],
    );
    let obj = out.as_object().expect("ack must be a JSON object");
    let mut keys: Vec<&str> = obj.keys().map(|k| k.as_str()).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        ["action", "legs", "limit_price", "order_id", "quantity", "status", "underlying"]
    );
    assert_eq!(obj["order_id"], json!(77));
    assert_eq!(obj["underlying"], json!("AAPL"));
    assert_eq!(obj["limit_price"], json!(2.5));
    let legs = obj["legs"].as_array().expect("legs must be an array");
    assert_eq!(legs.len(), 2);
    let leg0 = legs[0].as_object().expect("leg echo must be an object");
    let mut leg_keys: Vec<&str> = leg0.keys().map(|k| k.as_str()).collect();
    leg_keys.sort_unstable();
    assert_eq!(
        leg_keys,
        ["action", "conid", "expiry", "ratio", "right", "strike", "symbol"]
    );
    assert_eq!(leg0["conid"], json!(111));
    assert_eq!(legs[1]["conid"], json!(222));
    assert_eq!(legs[1]["action"], json!("SELL"));
}

// ---- the live double gate (offline; stk/option-orders parity) ----

const VALID_COMBO: &[&str] = &[
    "--format", "json", "option-combo", "--action", "BUY", "--qty", "1", "--limit", "0.05",
    "--leg", "BUY 1 AAPL 20260918 240 C",
    "--leg", "SELL 1 AAPL 20260918 250 C",
];

fn with(base: &[&str], extra: &[&str]) -> Vec<String> {
    base.iter().chain(extra.iter()).map(|s| s.to_string()).collect()
}

fn run_expect(args: Vec<String>, code: &str) {
    let args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    expect_error_code(omi(), &args, code);
}

#[test]
fn live_combo_without_env_is_config_error() {
    run_expect(with(VALID_COMBO, &["--live"]), "config");
}

#[test]
fn hand_set_live_port_without_env_is_also_gated() {
    run_expect(with(VALID_COMBO, &["--port", "4001"]), "config");
}

#[test]
fn paper_dead_port_is_connection_error() {
    run_expect(with(VALID_COMBO, &["--host", "127.0.0.1", "--port", "65000"]), "connection");
}

// ---- local validation (usage; precedes gate AND connect) ----

#[test]
fn one_leg_is_usage_error() {
    run_expect(
        with(
            &["--format", "json", "option-combo", "--action", "BUY", "--qty", "1", "--limit", "1"],
            &["--leg", "BUY 1 AAPL 20260918 240 C"],
        ),
        "usage",
    );
}

#[test]
fn five_legs_is_usage_error() {
    let mut args: Vec<String> =
        ["--format", "json", "option-combo", "--action", "BUY", "--qty", "1", "--limit", "1"]
            .iter()
            .map(|s| s.to_string())
            .collect();
    for strike in ["100", "110", "120", "130", "140"] {
        args.push("--leg".to_string());
        args.push(format!("BUY 1 AAPL 20260918 {strike} C"));
    }
    run_expect(args, "usage");
}

#[test]
fn mixed_underlyings_are_usage_error() {
    run_expect(
        with(
            &["--format", "json", "option-combo", "--action", "BUY", "--qty", "1", "--limit", "1"],
            &["--leg", "BUY 1 AAPL 20260918 240 C", "--leg", "SELL 1 MSFT 20260918 500 C"],
        ),
        "usage",
    );
}

#[test]
fn malformed_leg_is_usage_error_naming_the_leg() {
    let args = with(
        &["--format", "json", "option-combo", "--action", "BUY", "--qty", "1", "--limit", "1"],
        &["--leg", "BUY 1 AAPL 20260918 240 C", "--leg", "SELL 1 AAPL 20260918 240 X"],
    );
    let strs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let output = omi().args(&strs).assert().failure().get_output().clone();
    let stderr = String::from_utf8_lossy(&output.stderr);
    let v: Value = serde_json::from_str(stderr.trim()).expect("JSON error envelope");
    assert_eq!(v["error"]["code"], "usage");
    assert!(
        v["error"]["message"].as_str().unwrap_or_default().contains("leg 2"),
        "leg errors must name the 1-based leg index: {stderr}"
    );
}

#[test]
fn bad_action_is_usage_error() {
    run_expect(
        with(
            &["--format", "json", "option-combo", "--action", "HOLD", "--qty", "1", "--limit", "1"],
            &["--leg", "BUY 1 AAPL 20260918 240 C", "--leg", "SELL 1 AAPL 20260918 250 C"],
        ),
        "usage",
    );
}

#[test]
fn fractional_qty_is_usage_error() {
    run_expect(
        with(
            &["--format", "json", "option-combo", "--action", "BUY", "--qty", "1.5", "--limit", "1"],
            &["--leg", "BUY 1 AAPL 20260918 240 C", "--leg", "SELL 1 AAPL 20260918 250 C"],
        ),
        "usage",
    );
}

#[test]
fn non_finite_limit_is_usage_error() {
    run_expect(
        with(
            &["--format", "json", "option-combo", "--action", "BUY", "--qty", "1", "--limit", "NaN"],
            &["--leg", "BUY 1 AAPL 20260918 240 C", "--leg", "SELL 1 AAPL 20260918 250 C"],
        ),
        "usage",
    );
}

#[test]
fn validation_precedes_the_live_gate() {
    // usage < config: one leg WITH --live and no env must be usage, not config.
    run_expect(
        with(
            &["--format", "json", "option-combo", "--action", "BUY", "--qty", "1", "--limit", "1"],
            &["--leg", "BUY 1 AAPL 20260918 240 C", "--live"],
        ),
        "usage",
    );
}

#[test]
fn missing_args_are_usage_errors() {
    expect_error_code(omi(), &["--format", "json", "option-combo"], "usage");
}

// ---- CLI surface ----

#[test]
fn help_lists_option_combo() {
    omi()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("option-combo"));
}

#[test]
fn option_combo_help_succeeds() {
    omi().args(["option-combo", "--help"]).assert().success();
}
