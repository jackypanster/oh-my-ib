//! FROZEN SPEC — card 02 (the six read subcommands).
//! Black-box subprocess assertions only. Deterministic offline: connection
//! tests target a dead port. The coder must NOT edit this file (freeze gate).
//!
//! On an empty repo this is RED. It goes GREEN once impl registers the six
//! subcommands and the structured error envelope.

use assert_cmd::Command;
use predicates::prelude::*;

fn omi() -> Command {
    Command::cargo_bin("omi").expect("the `omi` binary should build")
}

#[test]
fn account_help_succeeds() {
    omi().args(["account", "--help"]).assert().success();
}

#[test]
fn positions_help_succeeds() {
    omi().args(["positions", "--help"]).assert().success();
}

#[test]
fn orders_help_succeeds() {
    omi().args(["orders", "--help"]).assert().success();
}

#[test]
fn contract_help_succeeds() {
    omi().args(["contract", "--help"]).assert().success();
}

#[test]
fn quote_help_mentions_md_type() {
    omi()
        .args(["quote", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--md-type"));
}

#[test]
fn history_help_mentions_bar_and_duration() {
    omi()
        .args(["history", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--bar"))
        .stdout(predicate::str::contains("--duration"));
}

#[test]
fn quote_connection_error_is_json_envelope() {
    let output = omi()
        .args([
            "--format", "json", "quote", "AAPL", "--host", "127.0.0.1", "--port", "65000",
        ])
        .assert()
        .failure()
        .get_output()
        .clone();
    let stderr = String::from_utf8_lossy(&output.stderr);
    let v: serde_json::Value = serde_json::from_str(stderr.trim())
        .expect("stderr must be a JSON error envelope");
    assert_eq!(v["error"]["code"], "connection");
}
