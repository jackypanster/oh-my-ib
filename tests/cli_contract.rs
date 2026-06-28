//! FROZEN SPEC — card 01 (core CLI + connection-error envelope).
//! Black-box subprocess assertions: reference ONLY the `omi` binary contract
//! (args, stdout/stderr, exit code) — never internal symbols. Deterministic
//! offline: the connection test targets a dead port, so no IB Gateway is needed.
//!
//! On an empty repo this is RED (no crate yet). pipeline-impl makes it GREEN by
//! creating Cargo.toml + src. The coder must NOT edit this file (the freeze gate).

use assert_cmd::Command;
use predicates::prelude::*;

fn omi() -> Command {
    Command::cargo_bin("omi").expect("the `omi` binary should build")
}

#[test]
fn help_lists_all_subcommands() {
    omi()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("health"))
        .stdout(predicate::str::contains("account"))
        .stdout(predicate::str::contains("positions"))
        .stdout(predicate::str::contains("orders"))
        .stdout(predicate::str::contains("quote"))
        .stdout(predicate::str::contains("contract"))
        .stdout(predicate::str::contains("history"));
}

#[test]
fn version_succeeds() {
    omi().arg("--version").assert().success();
}

#[test]
fn health_help_succeeds() {
    omi().args(["health", "--help"]).assert().success();
}

#[test]
fn unknown_subcommand_fails() {
    omi().arg("frobnicate").assert().failure();
}

#[test]
fn connection_error_is_json_envelope() {
    // Dead port => deterministic connection refusal, independent of any gateway.
    let output = omi()
        .args([
            "--format", "json", "health", "--host", "127.0.0.1", "--port", "65000",
        ])
        .assert()
        .failure()
        .get_output()
        .clone();
    let stderr = String::from_utf8_lossy(&output.stderr);
    let v: serde_json::Value = serde_json::from_str(stderr.trim())
        .expect("stderr must be a JSON error envelope");
    assert_eq!(
        v["error"]["code"], "connection",
        "connection failures must carry code=\"connection\""
    );
}
