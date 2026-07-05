//! FROZEN SPEC — preview-readonly (card 01). Offline. The coder must NOT edit this file.
//!
//! Freezes the READ-ONLY preview contract (ADR 0027): `--preview` NEVER transmits. It REPLACES the
//! whatIf spec (order-preview) — live-acceptance REFUTED R1 (Tiger TRANSMITS whatIf orders), so the
//! preview path drops `place_order`/`what_if` entirely.
//!   - pure `shape_preview(Value, &Order, multiplier, ccy)`: a `transmits:false` envelope with
//!     `notional` (qty×|limit|×multiplier; `null` for MKT), and NO `what_if`/`margin`/`commission`/
//!     `status` keys.
//!   - `--preview` accepted on all six order verbs (dead paper port ⇒ `connection`).
//!   - the REAL order path is STILL gated: `buy --live` without `OMI_ALLOW_LIVE` ⇒ `config`.
//!
//! RED until impl rewrites `shape_preview` to the read-only signature (the old
//! `(&Contract,&Order,&OrderState)` signature no longer resolves).
//!
//! NOT frozen (review-by-reading + cc live-acceptance — CONTEXT.md): the `contract_details` wiring;
//! CONTAINMENT (the preview path calls NO `place_order`); the read-shaped gate (`--live --preview`
//! without env reaches connect, not `config` — observable only on the live port, so cc verifies it);
//! and that `omi --live … --preview` leaves `omi --live orders` EMPTY (the R1-fix acceptance).

use assert_cmd::Command;
use ibapi::orders::Action;
use predicates::prelude::*;
use serde_json::{json, Value};

use oh_my_ib::ib::{build_stk_order, shape_preview};

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

// ---- pure READ-ONLY preview envelope (no transmit; notional; NO whatIf/margin/commission) ----

#[test]
fn preview_envelope_is_read_only_with_notional() {
    let (_c, order) = build_stk_order("AAPL", Action::Buy, 100.0, Some(290.0));
    let contract = json!({
        "symbol": "AAPL", "conid": 265598, "sec_type": "STK",
        "exchange": "SMART", "currency": "USD", "long_name": "APPLE INC"
    });
    let out = shape_preview(contract, &order, 1.0, "USD");
    let obj = out.as_object().expect("preview must be a JSON object");
    let mut keys: Vec<&str> = obj.keys().map(|k| k.as_str()).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "action", "contract", "note", "notional", "notional_currency", "order", "preview",
            "transmits"
        ]
    );
    assert_eq!(out["preview"], json!(true));
    assert_eq!(out["transmits"], json!(false), "read-only preview: transmits MUST be false");
    assert_eq!(out["notional"], json!(29000.0), "100 x 290 x 1");
    assert_eq!(out["notional_currency"], json!("USD"));
    assert_eq!(out["order"]["limit"], json!(290.0));
    assert_eq!(out["contract"]["conid"], json!(265598));
    // whatIf-era keys are GONE (the mechanism that transmitted on Tiger):
    assert!(out.get("what_if").is_none(), "read-only preview must not carry what_if");
    assert!(out.get("margin").is_none(), "no margin (whatIf transmits on this gateway)");
    assert!(out.get("commission").is_none(), "no commission (whatIf transmits)");
}

#[test]
fn preview_notional_uses_the_multiplier() {
    // notional = qty x |limit| x multiplier; options pass multiplier 100.
    let (_c, order) = build_stk_order("X", Action::Buy, 2.0, Some(5.0));
    let out = shape_preview(json!({"sec_type": "OPT"}), &order, 100.0, "USD");
    assert_eq!(out["notional"], json!(1000.0), "2 x 5 x 100");
}

#[test]
fn preview_mkt_has_null_notional() {
    // MKT (no limit) ⇒ notional cannot be computed ⇒ null (key present).
    let (_c, order) = build_stk_order("MSFT", Action::Sell, 5.0, None);
    let out = shape_preview(json!({"sec_type": "STK"}), &order, 1.0, "USD");
    assert_eq!(out["notional"], Value::Null, "MKT preview: no limit ⇒ notional null");
}

// ---- --preview accepted on ALL SIX verbs (dead paper port ⇒ connection, past parse) ----

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

// ---- the REAL order path is STILL gated (errors before any connect — robust, gateway-independent) ----

#[test]
fn real_buy_on_live_without_env_is_still_config_error() {
    expect_error_code(
        omi(),
        &["--format", "json", "buy", "AAPL", "1", "--limit", "1", "--live"],
        "config",
    );
}

#[test]
fn real_sell_on_live_without_env_is_still_config_error() {
    expect_error_code(omi(), &["--format", "json", "sell", "AAPL", "1", "--live"], "config");
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
