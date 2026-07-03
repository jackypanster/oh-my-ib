//! FROZEN SPEC — brief-command (card 01). Offline. The coder must NOT edit this file.
//!
//! Freezes the black-box CLI contract of `omi brief` + the pure assembly seam `assemble_brief`
//! (account + as_of + six section payloads → ONE composite JSON object, exact 8-key top level,
//! pass-through — no re-shaping, no key invention). RED until impl adds the `Brief` subcommand and
//! re-exports `oh_my_ib::ib::assemble_brief` (the import below won't resolve, and `--help` won't
//! list `brief`, until then).
//!
//! NOT frozen (reviewed-by-reading + operator live acceptance per PRD criterion 10): the
//! `brief(cfg)` gateway fn — ADR 0010 fixed fetch order (resolve → as_of → consolidated
//! account_updates drain → pnl take-first → pnl_single sweep → open-orders drain → executions
//! drain, strictly sequential consume-then-drop), ADR 0011 shared builders, as_of ISO-8601
//! formatting, fail-fast no-partial mapping, `--format table`, and the six sibling seam
//! refactors' behavior preservation.

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::{json, Value};

use oh_my_ib::ib::assemble_brief;

fn omi() -> Command {
    Command::cargo_bin("omi").expect("the `omi` binary should build")
}

// ---- fixtures (plain serde_json Values; NO ibapi types) ----

fn sample_sections() -> (Value, Value, Value, Value, Value, Value) {
    let account_summary = json!({
        "net_liquidation": 100000.5, "total_cash": 25000.0, "buying_power": 200000.0,
        "available_funds": 99000.0, "currency": "USD"
    });
    let pnl = json!({ "daily_pnl": 12.5, "unrealized_pnl": null, "realized_pnl": -3.25 });
    let pnl_by_position = json!([
        { "conid": 265598, "symbol": "AAPL", "position": 100.0,
          "daily_pnl": 52.3, "unrealized_pnl": 1204.5, "realized_pnl": null, "value": 21050.0 }
    ]);
    let positions = json!([
        { "symbol": "AAPL", "conid": 265598, "qty": 100.0, "avg_cost": 190.2,
          "market_price": 210.5, "market_value": 21050.0, "unrealized_pnl": 1204.5,
          "realized_pnl": 0.0, "account": "TEST" }
    ]);
    let orders = json!([
        { "order_id": 7, "account": "TEST", "symbol": "MSFT", "conid": 272093,
          "action": "Buy", "quantity": 10.0, "order_type": "LMT", "limit_price": 400.0,
          "aux_price": null, "tif": "Day" }
    ]);
    let executions = json!([
        { "exec_id": "0001", "order_id": 7, "perm_id": 99, "time": "20260703 10:00:00",
          "symbol": "MSFT", "conid": 272093, "side": "BOT", "shares": 10.0, "price": 399.5,
          "cumulative_qty": 10.0, "avg_price": 399.5, "exchange": "SMART",
          "commission": 1.0, "commission_currency": "USD", "realized_pnl": null }
    ]);
    (account_summary, pnl, pnl_by_position, positions, orders, executions)
}

// ---- black-box CLI contract ----

#[test]
fn help_lists_brief_subcommand() {
    omi()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("brief"));
}

#[test]
fn brief_help_succeeds() {
    omi().args(["brief", "--help"]).assert().success();
}

#[test]
fn brief_dead_port_is_connection_error_envelope() {
    // Dead port => deterministic connection refusal, independent of any gateway. Fail-fast:
    // non-zero exit, structured envelope on stderr (PRD criteria 5/7).
    let output = omi()
        .args(["--format", "json", "brief", "--host", "127.0.0.1", "--port", "65000"])
        .assert()
        .failure()
        .get_output()
        .clone();
    let stderr = String::from_utf8_lossy(&output.stderr);
    let v: Value =
        serde_json::from_str(stderr.trim()).expect("stderr must be a JSON error envelope");
    assert_eq!(
        v["error"]["code"], "connection",
        "connection failures must carry code=\"connection\""
    );
}

// ---- pure assembly seam (account + as_of + six sections → composite) ----

#[test]
fn composite_has_exactly_the_eight_top_level_keys() {
    let (s, p, pbp, pos, ord, ex) = sample_sections();
    let out = assemble_brief("TEST", "2026-07-03T02:06:15Z", s, p, pbp, pos, ord, ex);
    let obj = out.as_object().expect("brief must be a JSON object");
    let mut keys: Vec<&str> = obj.keys().map(|k| k.as_str()).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "account",
            "account_summary",
            "as_of",
            "executions",
            "orders",
            "pnl",
            "pnl_by_position",
            "positions"
        ]
    );
}

#[test]
fn account_and_as_of_pass_through() {
    let (s, p, pbp, pos, ord, ex) = sample_sections();
    let out = assemble_brief("U1234567", "2026-07-03T02:06:15Z", s, p, pbp, pos, ord, ex);
    assert_eq!(out["account"], json!("U1234567"));
    assert_eq!(out["as_of"], json!("2026-07-03T02:06:15Z"));
}

#[test]
fn sections_pass_through_unmodified() {
    // The seam must NOT re-shape: each section lands byte-identical under its key (PRD
    // criterion 2 — the hoisting rule lives in the callers, not here).
    let (s, p, pbp, pos, ord, ex) = sample_sections();
    let out = assemble_brief(
        "TEST",
        "2026-07-03T02:06:15Z",
        s.clone(),
        p.clone(),
        pbp.clone(),
        pos.clone(),
        ord.clone(),
        ex.clone(),
    );
    assert_eq!(out["account_summary"], s);
    assert_eq!(out["pnl"], p);
    assert_eq!(out["pnl_by_position"], pbp);
    assert_eq!(out["positions"], pos);
    assert_eq!(out["orders"], ord);
    assert_eq!(out["executions"], ex);
}

#[test]
fn quiet_account_empty_sections_pass_through() {
    // Flat/quiet account: [] sections stay [] (PRD criterion 6) — never null, never dropped.
    let out = assemble_brief(
        "TEST",
        "2026-07-03T02:06:15Z",
        json!({ "net_liquidation": 0.0, "total_cash": 0.0, "buying_power": 0.0,
                "available_funds": 0.0, "currency": "USD" }),
        json!({ "daily_pnl": 0.0, "unrealized_pnl": null, "realized_pnl": null }),
        json!([]),
        json!([]),
        json!([]),
        json!([]),
    );
    assert_eq!(out["pnl_by_position"], json!([]));
    assert_eq!(out["positions"], json!([]));
    assert_eq!(out["orders"], json!([]));
    assert_eq!(out["executions"], json!([]));
}
