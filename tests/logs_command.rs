//! FROZEN SPEC — card 02 (invocation audit JSONL + `omi logs`).
//! Black-box subprocess assertions; deterministic offline (dead port 65000 per
//! the cli_contract convention). Every test overrides HOME to a private temp dir
//! — the audit path derives from $HOME (ADR 0036) — so the real
//! ~/.local/share/oh-my-ib is never touched.
//!
//! RED today: no audit seam exists and `omi logs` is an unknown subcommand.
//! pipeline-impl makes it GREEN via src/audit.rs + the main.rs seam + the Logs
//! variant (arch.md, ADR 0036/0037). Depends on card 01 landing first (same
//! feature branch): the fail-open test drives `omi help`.
//! The coder must NOT edit this file (the freeze gate).

use assert_cmd::Command;

fn omi() -> Command {
    Command::cargo_bin("omi").expect("the `omi` binary should build")
}

fn temp_home(tag: &str) -> std::path::PathBuf {
    let home = std::env::temp_dir().join(format!("omi-spec-logs-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).expect("create temp HOME");
    home
}

fn audit_path(home: &std::path::Path) -> std::path::PathBuf {
    home.join(".local/share/oh-my-ib/invocations.jsonl")
}

/// One failed invocation ⇒ exactly one audit line, with the ADR 0036 field set,
/// and the --account value redacted even in the local file.
#[test]
fn failed_invocation_appends_one_redacted_audit_line() {
    let home = temp_home("append");
    omi()
        .env("HOME", &home)
        .args([
            "health",
            "--host",
            "127.0.0.1",
            "--port",
            "65000",
            "--account",
            "SECRETACC",
        ])
        .assert()
        .failure();
    let raw = std::fs::read_to_string(audit_path(&home)).expect("invocations.jsonl should exist");
    let lines: Vec<&str> = raw.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(lines.len(), 1, "exactly one audit line expected");
    let entry: serde_json::Value =
        serde_json::from_str(lines[0]).expect("audit line should be valid JSON");
    assert_eq!(entry["cmd"], "health");
    assert_eq!(entry["mode"], "paper");
    assert_eq!(entry["exit"], 2);
    assert_eq!(entry["error"], "connection");
    assert!(entry["ts"].is_string(), "ts missing");
    assert!(entry["duration_ms"].is_number(), "duration_ms missing");
    assert!(!raw.contains("SECRETACC"), "--account value must be redacted");
    assert!(raw.contains("***"), "redaction placeholder expected");
}

/// `omi logs --tail N` reads the entries back (newest last) with the reader's
/// envelope: path + entries + skipped_malformed.
#[test]
fn logs_tail_returns_the_recorded_entry() {
    let home = temp_home("tail");
    omi()
        .env("HOME", &home)
        .args(["health", "--host", "127.0.0.1", "--port", "65000"])
        .assert()
        .failure();
    let assert = omi()
        .env("HOME", &home)
        .args(["logs", "--tail", "1"])
        .assert()
        .success();
    let v: serde_json::Value = serde_json::from_slice(&assert.get_output().stdout)
        .expect("`omi logs` stdout should be valid JSON");
    assert!(
        v["path"]
            .as_str()
            .expect("path string")
            .ends_with(".local/share/oh-my-ib/invocations.jsonl"),
        "path should expose the audit file location"
    );
    let entries = v["entries"].as_array().expect("entries array");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["cmd"], "health");
    assert_eq!(v["skipped_malformed"], 0);
}

/// No audit file yet ⇒ `omi logs` is an empty success, not an error.
#[test]
fn logs_with_no_file_is_empty_success() {
    let home = temp_home("empty");
    let assert = omi().env("HOME", &home).arg("logs").assert().success();
    let v: serde_json::Value = serde_json::from_slice(&assert.get_output().stdout)
        .expect("`omi logs` stdout should be valid JSON");
    assert_eq!(v["entries"].as_array().expect("entries array").len(), 0);
    assert_eq!(v["skipped_malformed"], 0);
}

/// ADR 0037 fail-open: an unwritable HOME must not change the command's own
/// result — stdout stays clean JSON, exit stays 0, and stderr carries a plain
/// `warn:` line that is NOT the JSON error envelope.
#[test]
fn audit_write_failure_is_fail_open() {
    let dir = temp_home("failopen");
    let home_file = dir.join("home-is-a-file");
    std::fs::write(&home_file, b"not a dir").expect("write blocker file");
    let assert = omi().env("HOME", &home_file).arg("help").assert().success();
    let out = assert.get_output();
    let stdout: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout should stay valid JSON");
    assert!(stdout["commands"].is_array());
    let stderr = String::from_utf8(out.stderr.clone()).expect("stderr utf8");
    assert!(stderr.contains("warn:"), "expected a warn: line, got: {stderr}");
    assert!(
        !stderr.contains("\"error\""),
        "the warn must NOT be the JSON error envelope: {stderr}"
    );
}
