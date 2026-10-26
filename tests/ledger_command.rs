//! `omi ledger` CLI contract. Offline: `--help`, `--days` bounds, and a dead
//! gateway (connection error — not a silent empty ledger). Shaping of
//! supported / unsupported / partial lives in `src/ib/ledger.rs` unit tests.

use assert_cmd::Command;
use serde_json::Value;

fn omi() -> Command {
    Command::cargo_bin("omi").expect("the `omi` binary should build")
}

fn isolated_home() -> std::path::PathBuf {
    let home = std::env::temp_dir().join(format!(
        "omi-ledger-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    std::fs::create_dir_all(&home).expect("create temp HOME");
    home
}

fn stderr_json(args: &[&str]) -> (i32, Value) {
    let home = isolated_home();
    let assert = omi().env("HOME", &home).args(args).assert().failure();
    let code = assert.get_output().status.code().expect("exit code");
    let v: Value = serde_json::from_slice(&assert.get_output().stderr)
        .expect("stderr should be a JSON error envelope");
    (code, v)
}

#[test]
fn ledger_help_documents_days() {
    let assert = omi().args(["ledger", "--help"]).assert().success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(stdout.contains("--days"), "{stdout}");
    assert!(stdout.contains("90"), "{stdout}");
    assert!(
        stdout.contains("7"),
        "default lookback 7 must be documented: {stdout}"
    );
}

#[test]
fn days_out_of_range_is_usage_before_connect() {
    for days in ["0", "91"] {
        let (code, v) = stderr_json(&["--format", "json", "ledger", "--days", days]);
        assert_eq!(code, 64, "days={days} body={v}");
        assert_eq!(v["error"]["code"], "usage");
        assert!(
            v["error"]["message"]
                .as_str()
                .unwrap_or("")
                .contains("1 to 90"),
            "{v}"
        );
    }
}

#[test]
fn dead_gateway_is_connection_not_an_empty_ledger() {
    let home = isolated_home();
    let assert = omi()
        .env("HOME", &home)
        .args([
            "--format",
            "json",
            "--host",
            "127.0.0.1",
            "--port",
            "1",
            "ledger",
            "--days",
            "7",
        ])
        .assert()
        .failure();
    let code = assert.get_output().status.code().expect("exit code");
    assert_eq!(
        code,
        2,
        "stderr={}",
        String::from_utf8_lossy(&assert.get_output().stderr)
    );
    let v: Value = serde_json::from_slice(&assert.get_output().stderr).expect("json stderr");
    assert_eq!(v["error"]["code"], "connection");
    assert!(
        assert.get_output().stdout.is_empty(),
        "a failed connect must not print an empty entries success"
    );
}
