//! FROZEN SPEC — order-preview (card 01). Offline. The coder must NOT edit this file.
//!
//! Freezes the whatIf order-preview contract (ADR 0026):
//!   - the pure preview-envelope seam `shape_preview(&Contract, &Order, &OrderState) -> Value`
//!     (uniform 9-key envelope; `Option<f64>::None` -> JSON null; margin/commission mapping),
//!   - the real transmit path stays `what_if = false` (asserted on `build_stk_order` output),
//!   - the CLI/gate contract: `--preview` is accepted on ALL SIX order verbs, gated IDENTICALLY to a
//!     real order (live without `OMI_ALLOW_LIVE` => `config` error), and listed in `--help`.
//! RED until impl adds the `--preview` flag (`GlobalOpts` -> `Config`), the `preview_with_client`
//! gateway fn, and re-exports `oh_my_ib::ib::shape_preview`.
//!
//! NOT frozen (reviewed-by-reading + operator live-acceptance, CONTEXT.md R1/R2): the gateway fn
//! `preview_with_client` — that it sets `Order.what_if = true`, reads `OpenOrder.order_state`, and
//! that whatIf does NOT transmit on the Tiger gateway. Write calls stay ONLY in `src/ib/trade.rs`.
//! The dead-port cases below assert the CLI/gate wiring up to the connection boundary; the envelope
//! shape itself is frozen via the pure `shape_preview` seam (no gateway needed).

use assert_cmd::Command;
use ibapi::orders::{Action, OrderState};
use predicates::prelude::*;
use serde_json::{json, Value};

use oh_my_ib::ib::{build_stk_order, shape_preview};

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

// ---- pure preview-envelope seam (Contract + Order + OrderState -> uniform envelope) ----

#[test]
fn preview_envelope_has_the_uniform_keys_and_maps_orderstate() {
    let (contract, order) = build_stk_order("AAPL", Action::Buy, 100.0, Some(250.0));
    let state = OrderState {
        initial_margin_change: Some(1234.5),
        commission: Some(1.0),
        commission_currency: "USD".to_string(),
        ..Default::default()
    };
    let out = shape_preview(&contract, &order, &state);
    let obj = out.as_object().expect("preview must be a JSON object");
    let mut keys: Vec<&str> = obj.keys().map(|k| k.as_str()).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "action", "commission", "contract", "margin", "order", "preview", "status", "warning",
            "what_if"
        ]
    );
    assert_eq!(out["preview"], json!(true));
    assert_eq!(out["what_if"], json!(true));
    assert_eq!(out["margin"]["init_change"], json!(1234.5));
    assert_eq!(out["commission"]["value"], json!(1.0));
    assert_eq!(out["commission"]["currency"], json!("USD"));
    assert_eq!(out["order"]["type"], json!("LMT"));
    assert_eq!(out["order"]["qty"], json!(100.0));
    assert_eq!(out["order"]["limit"], json!(250.0));
    assert_eq!(out["contract"]["symbol"], json!("AAPL"));
}

#[test]
fn preview_absent_orderstate_fields_are_json_null_not_missing() {
    // Tiger may honor what_if but leave margin/commission empty (CONTEXT.md R2): None -> null,
    // key present. The envelope stays a valid confirm card (echo + resolved contract).
    let (contract, order) = build_stk_order("MSFT", Action::Sell, 5.0, None);
    let state = OrderState::default();
    let out = shape_preview(&contract, &order, &state);
    assert_eq!(out["margin"]["init_change"], Value::Null, "None -> null, key present");
    assert_eq!(out["margin"]["maint_change"], Value::Null);
    assert_eq!(out["commission"]["value"], Value::Null);
    // MKT preview (no limit): limit echoes as null, key present.
    assert_eq!(out["order"]["limit"], Value::Null);
}

// ---- the real transmit path must never flip what_if ----

#[test]
fn real_order_build_keeps_what_if_false() {
    let (_contract, order) = build_stk_order("AAPL", Action::Buy, 1.0, Some(1.0));
    assert!(!order.what_if, "the real transmit path must never set what_if");
}

// ---- CLI/gate contract: --preview accepted on ALL SIX verbs, gated like a real order ----
// Dead paper port (65000) is ungated: past parse+gate, the command reaches connect and fails with
// `connection`. Today `--preview` is an unknown arg => `usage` (RED) until impl adds the flag.

#[test]
fn preview_accepted_on_buy() {
    expect_error_code(
        omi(),
        &["--format", "json", "buy", "AAPL", "1", "--limit", "1", "--host", "127.0.0.1", "--port", "65000", "--preview"],
        "connection",
    );
}

#[test]
fn preview_accepted_on_sell() {
    expect_error_code(
        omi(),
        &["--format", "json", "sell", "AAPL", "1", "--host", "127.0.0.1", "--port", "65000", "--preview"],
        "connection",
    );
}

#[test]
fn preview_accepted_on_option_buy() {
    expect_error_code(
        omi(),
        &["--format", "json", "option-buy", "--symbol", "AAPL", "--expiry", "20260918", "--strike", "250", "--right", "C", "--qty", "1", "--limit", "5", "--host", "127.0.0.1", "--port", "65000", "--preview"],
        "connection",
    );
}

#[test]
fn preview_accepted_on_option_sell() {
    expect_error_code(
        omi(),
        &["--format", "json", "option-sell", "--symbol", "AAPL", "--expiry", "20260918", "--strike", "250", "--right", "P", "--qty", "1", "--limit", "5", "--host", "127.0.0.1", "--port", "65000", "--preview"],
        "connection",
    );
}

#[test]
fn preview_accepted_on_option_combo() {
    expect_error_code(
        omi(),
        &["--format", "json", "option-combo", "--action", "BUY", "--qty", "1", "--limit", "0.05", "--leg", "BUY 1 AAPL 20260918 240 C", "--leg", "SELL 1 AAPL 20260918 250 C", "--host", "127.0.0.1", "--port", "65000", "--preview"],
        "connection",
    );
}

#[test]
fn preview_accepted_on_option_close() {
    expect_error_code(
        omi(),
        &["--format", "json", "option-close", "--conid", "123456789", "--limit", "3.2", "--host", "127.0.0.1", "--port", "65000", "--preview"],
        "connection",
    );
}

// ---- gate is IDENTICAL to a real order: live without OMI_ALLOW_LIVE is a config error (no connect) ----

#[test]
fn preview_on_live_without_env_is_config_error_like_a_real_order() {
    expect_error_code(
        omi(),
        &["--format", "json", "buy", "AAPL", "1", "--limit", "1", "--live", "--preview"],
        "config",
    );
}

// ---- the flag is documented ----

#[test]
fn buy_help_lists_the_preview_flag() {
    omi()
        .args(["buy", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--preview"));
}
