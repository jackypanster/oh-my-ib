//! FROZEN SPEC — option-close card 01. Offline. The coder must NOT edit this file.
//!
//! Freezes the `position_row` identity row (ADR 0022 §4): the FIRST-ever freeze of this
//! shape (it was pub(crate) — unreachable from tests/ — until this card promotes it).
//! Exact 14 keys = the legacy 9 (`symbol, conid, qty, avg_cost, market_price, market_value,
//! unrealized_pnl, realized_pnl, account`, values untouched) + 5 identity keys:
//! `sec_type` (ALWAYS a string — the IB wire code via SecurityType Display, "STK"/"OPT") and
//! `expiry`/`strike`/`right`/`multiplier` (populated iff the contract is an option, else
//! ALL null; empty multiplier string ⇒ null; right maps Call⇒"C", Put⇒"P").
//! `brief` parity is automatic (same fn) — not separately frozen.
//! RED until impl promotes `position_row` to pub and re-exports `oh_my_ib::ib::position_row`.
//!
//! NOT frozen (reviewed-by-reading + paper acceptance): the `positions`/`brief` gateway
//! drains (unchanged), Tiger's actual portfolio field content (PRD risk 1 — acceptance
//! observes), and the non_exhaustive-right fallback arm (unconstructible from outside ibapi).

use ibapi::accounts::AccountPortfolioValue;
use ibapi::contracts::Contract;
use serde_json::{json, Value};

use oh_my_ib::ib::position_row;

/// The exact frozen key set, sorted.
const KEYS: [&str; 14] = [
    "account", "avg_cost", "conid", "expiry", "market_price", "market_value", "multiplier",
    "qty", "realized_pnl", "right", "sec_type", "strike", "symbol", "unrealized_pnl",
];

fn keys_of(row: &Value) -> Vec<&str> {
    let mut keys: Vec<&str> = row
        .as_object()
        .expect("row must be a JSON object")
        .keys()
        .map(|k| k.as_str())
        .collect();
    keys.sort_unstable();
    keys
}

fn stk_position() -> AccountPortfolioValue {
    let mut contract = Contract::stock("AAPL").build();
    contract.contract_id = 265598;
    AccountPortfolioValue {
        contract,
        position: 100.0,
        market_price: 210.5,
        market_value: 21050.0,
        average_cost: 150.25,
        unrealized_pnl: 6025.0,
        realized_pnl: 0.0,
        account: Some("DU1234567".to_string()),
    }
}

fn call_position(position: f64) -> AccountPortfolioValue {
    // Same builder chain the write path uses — the realistic decoded shape.
    let mut contract = Contract::call("AAPL")
        .strike(240.0)
        .expires_on(2026, 9, 18)
        .build();
    contract.contract_id = 495512569;
    AccountPortfolioValue {
        contract,
        position,
        market_price: 3.2,
        market_value: 320.0 * position,
        average_cost: 250.0,
        unrealized_pnl: 70.0,
        realized_pnl: 0.0,
        account: Some("DU1234567".to_string()),
    }
}

// ---- non-option rows: sec_type always present, the 4 option keys all null ----

#[test]
fn stk_row_has_exactly_14_keys_with_null_option_identity() {
    let row = position_row(&stk_position());
    assert_eq!(keys_of(&row), KEYS);
    assert_eq!(row["sec_type"], json!("STK"), "IB wire code via Display, not Debug");
    assert_eq!(row["expiry"], Value::Null);
    assert_eq!(row["strike"], Value::Null);
    assert_eq!(row["right"], Value::Null);
    assert_eq!(row["multiplier"], Value::Null);
}

#[test]
fn legacy_nine_keys_are_byte_identical_for_stk() {
    let row = position_row(&stk_position());
    assert_eq!(row["symbol"], json!("AAPL"));
    assert_eq!(row["conid"], json!(265598));
    assert_eq!(row["qty"], json!(100.0));
    assert_eq!(row["avg_cost"], json!(150.25));
    assert_eq!(row["market_price"], json!(210.5));
    assert_eq!(row["market_value"], json!(21050.0));
    assert_eq!(row["unrealized_pnl"], json!(6025.0));
    assert_eq!(row["realized_pnl"], json!(0.0));
    assert_eq!(row["account"], json!("DU1234567"));
}

// ---- option rows: full identity, long and short ----

#[test]
fn long_call_row_carries_full_identity() {
    let row = position_row(&call_position(2.0));
    assert_eq!(keys_of(&row), KEYS);
    assert_eq!(row["sec_type"], json!("OPT"));
    assert_eq!(row["expiry"], json!("20260918"), "raw YYYYMMDD passthrough");
    assert_eq!(row["strike"], json!(240.0));
    assert_eq!(row["right"], json!("C"));
    assert_eq!(row["multiplier"], json!("100"), "string passthrough (house style)");
    assert_eq!(row["conid"], json!(495512569));
    assert_eq!(row["qty"], json!(2.0));
}

#[test]
fn short_put_row_mirrors_and_preserves_the_sign() {
    let mut v = call_position(-1.0);
    v.contract = Contract::put("AAPL")
        .strike(240.0)
        .expires_on(2026, 9, 18)
        .build();
    v.contract.contract_id = 495512570;
    let row = position_row(&v);
    assert_eq!(row["sec_type"], json!("OPT"));
    assert_eq!(row["right"], json!("P"));
    assert_eq!(
        row["qty"],
        json!(-1.0),
        "the sign is the close-side authority (ADR 0022 §2) — never abs()'d here"
    );
}

#[test]
fn empty_multiplier_on_an_option_row_is_null_not_empty_string() {
    let mut v = call_position(1.0);
    v.contract.multiplier = String::new();
    let row = position_row(&v);
    assert_eq!(row["multiplier"], Value::Null);
    assert_eq!(row["sec_type"], json!("OPT"), "the other identity keys are unaffected");
    assert_eq!(row["expiry"], json!("20260918"));
}
