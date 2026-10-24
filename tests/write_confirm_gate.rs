//! Write verbs are preview-by-default. `--confirm` is the send switch; the global
//! `--preview` flag forces the preview even when `--confirm` is set.
//!
//! Offline. Dead paper port (65000) stands in for "got past parse, tried to connect".
//! A live port without `OMI_ALLOW_LIVE` must stay `config` on the preview arm for
//! every verb except `reprice` (that skip is frozen in `tests/reprice.rs`).

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;

use oh_my_ib::ib::{shape_cancel_preview, write_transmits};

fn omi() -> Command {
    let mut cmd = Command::cargo_bin("omi").expect("the `omi` binary should build");
    cmd.env_remove("OMI_ALLOW_LIVE");
    cmd.env_remove("OMI_MAX_NOTIONAL");
    cmd
}

fn stderr_json(cmd: &mut Command) -> (i32, Value) {
    let output = cmd.assert().failure().get_output().clone();
    let stderr = String::from_utf8_lossy(&output.stderr);
    let v: Value = serde_json::from_str(stderr.trim()).expect("stderr must be a JSON envelope");
    (output.status.code().unwrap_or(-1), v)
}

fn dead_paper(args: &[&str]) -> Vec<String> {
    let mut v: Vec<String> = args.iter().map(|s| (*s).to_string()).collect();
    v.extend(["--host".into(), "127.0.0.1".into(), "--port".into(), "65000".into()]);
    v
}

// ---- the shared transmit decision ------------------------------------------------

#[test]
fn write_transmits_is_confirm_and_not_global_preview() {
    assert!(write_transmits(true, false), "--confirm alone transmits");
    assert!(!write_transmits(true, true), "global --preview wins over --confirm");
    assert!(!write_transmits(false, false), "no --confirm is a preview");
    assert!(!write_transmits(false, true));
}

#[test]
fn cancel_preview_names_the_id_and_does_not_transmit() {
    let out = shape_cancel_preview(42);
    assert_eq!(out["preview"], true);
    assert_eq!(out["transmits"], false);
    assert_eq!(out["order_id"], 42);
    assert_eq!(out["action"], "cancel");
}

// ---- no --confirm: preview path. paper reaches connect; live is still the gate ----

#[test]
fn buy_without_confirm_on_dead_paper_reaches_connect() {
    let args = dead_paper(&["--format", "json", "buy", "AAPL", "1", "--limit", "1"]);
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let (code, v) = stderr_json(omi().args(&refs));
    assert_eq!(code, 2, "{v}");
    assert_eq!(v["error"]["code"], "connection");
}

#[test]
fn buy_without_confirm_on_live_is_still_config() {
    let (_, v) = stderr_json(omi().args(["--format", "json", "--live", "buy", "AAPL", "1", "--limit", "1"]));
    assert_eq!(v["error"]["code"], "config");
}

#[test]
fn sell_without_confirm_on_live_is_still_config() {
    let (_, v) = stderr_json(omi().args(["--format", "json", "--live", "sell", "AAPL", "1"]));
    assert_eq!(v["error"]["code"], "config");
}

#[test]
fn cancel_without_confirm_is_a_local_preview() {
    // No host/port: a preview cancel never connects, so a dead port is not required.
    let output = omi()
        .args(["--format", "json", "cancel", "42"])
        .assert()
        .success()
        .get_output()
        .clone();
    let v: Value = serde_json::from_slice(&output.stdout).expect("stdout JSON");
    assert_eq!(v["transmits"], false);
    assert_eq!(v["preview"], true);
    assert_eq!(v["order_id"], 42);
    assert_eq!(v["action"], "cancel");
}

#[test]
fn cancel_without_confirm_on_live_is_still_config() {
    let (_, v) = stderr_json(omi().args(["--format", "json", "--live", "cancel", "42"]));
    assert_eq!(v["error"]["code"], "config");
}

#[test]
fn option_buy_without_confirm_on_dead_paper_reaches_connect() {
    let args = dead_paper(&[
        "--format", "json", "option-buy", "--symbol", "AAPL", "--expiry", "20260918",
        "--strike", "250", "--right", "C", "--qty", "1", "--limit", "5",
    ]);
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let (_, v) = stderr_json(omi().args(&refs));
    assert_eq!(v["error"]["code"], "connection");
}

#[test]
fn option_combo_without_confirm_on_live_is_still_config() {
    let (_, v) = stderr_json(omi().args([
        "--format", "json", "--live", "option-combo", "--action", "BUY", "--qty", "1",
        "--limit", "0.05", "--leg", "BUY 1 AAPL 20260918 240 C", "--leg", "SELL 1 AAPL 20260918 250 C",
    ]));
    assert_eq!(v["error"]["code"], "config");
}

#[test]
fn option_close_without_confirm_on_live_is_still_config() {
    let (_, v) = stderr_json(omi().args([
        "--format", "json", "--live", "option-close", "--conid", "123", "--limit", "1",
    ]));
    assert_eq!(v["error"]["code"], "config");
}

// ---- --confirm is the real-send path (dead paper ⇒ connection; live ⇒ config) ----

#[test]
fn buy_confirm_on_dead_paper_reaches_connect() {
    let args = dead_paper(&["--format", "json", "buy", "AAPL", "1", "--limit", "1", "--confirm"]);
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let (_, v) = stderr_json(omi().args(&refs));
    assert_eq!(v["error"]["code"], "connection");
}

#[test]
fn cancel_confirm_on_dead_paper_reaches_connect() {
    let args = dead_paper(&["--format", "json", "cancel", "42", "--confirm"]);
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let (_, v) = stderr_json(omi().args(&refs));
    assert_eq!(v["error"]["code"], "connection");
}

#[test]
fn option_sell_confirm_on_live_without_env_is_config() {
    let (_, v) = stderr_json(omi().args([
        "--format", "json", "--live", "option-sell", "--symbol", "AAPL", "--expiry", "20260918",
        "--strike", "250", "--right", "P", "--qty", "1", "--limit", "5", "--confirm",
    ]));
    assert_eq!(v["error"]["code"], "config");
}

// ---- global --preview wins over --confirm. live port stays config (not reprice) ----

#[test]
fn buy_global_preview_overrides_confirm_but_live_gate_still_fires() {
    let (_, v) = stderr_json(omi().args([
        "--format", "json", "--preview", "--live", "buy", "AAPL", "1", "--limit", "1", "--confirm",
    ]));
    assert_eq!(v["error"]["code"], "config");
}

#[test]
fn cancel_global_preview_overrides_confirm_on_paper() {
    // Paper + global --preview: preview even with --confirm, and cancel preview
    // never connects, so this succeeds with transmits:false.
    let output = omi()
        .args(["--format", "json", "--preview", "cancel", "7", "--confirm"])
        .assert()
        .success()
        .get_output()
        .clone();
    let v: Value = serde_json::from_slice(&output.stdout).expect("stdout JSON");
    assert_eq!(v["transmits"], false);
    assert_eq!(v["order_id"], 7);
}

#[test]
fn option_combo_global_preview_overrides_confirm_on_dead_paper() {
    let args = dead_paper(&[
        "--format", "json", "--preview", "option-combo", "--action", "BUY", "--qty", "1",
        "--limit", "0.05", "--leg", "BUY 1 AAPL 20260918 240 C", "--leg", "SELL 1 AAPL 20260918 250 C",
        "--confirm",
    ]);
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let (_, v) = stderr_json(omi().args(&refs));
    assert_eq!(v["error"]["code"], "connection", "preview still resolves legs, so it connects");
}

// ---- help names the send switch --------------------------------------------------

#[test]
fn write_verb_help_names_confirm() {
    for verb in ["buy", "sell", "cancel", "option-buy", "option-sell", "option-combo", "option-close", "covered-call", "short-put"] {
        omi()
            .args([verb, "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("--confirm"));
    }
}
