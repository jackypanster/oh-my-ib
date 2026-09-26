//! FROZEN SPEC — snapshot-timeout (card 01). Offline. The coder must NOT edit this file.
//!
//! Freezes the pure seams of ADR 0038: the shared `SNAPSHOT_DEADLINE` const (20s, a TOTAL
//! per-snapshot deadline that must exceed IB's documented 11s snapshot tail and must not be the
//! 10s `TAKE_FIRST_TIMEOUT`), and the pure `snapshot_timeout_error` builder (code `timeout`,
//! exit 6, instrument-naming single-line message with the md-type and the delayed hint). Also
//! pins regressions: dead ports stay `code="connection"` and no timeout flag is added.
//! RED until impl exports `oh_my_ib::ib::{SNAPSHOT_DEADLINE, snapshot_timeout_error}`.
//!
//! NOT frozen (review-by-reading + operator live acceptance, arch.md §Freeze coverage): the
//! total-deadline loop swap in `quote_one` / `option_quote`, Notice skipping, Instant-classified
//! `None`, CancelMktData on drop, and label wiring. Triggering a silent snapshot offline needs a
//! fake IB server, which the no-mock rule forbids.

use std::time::Duration;

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;

use oh_my_ib::ib::{snapshot_timeout_error, SNAPSHOT_DEADLINE, TAKE_FIRST_TIMEOUT};

fn omi() -> Command {
    Command::cargo_bin("omi").expect("the `omi` binary should build")
}

// ---- the deadline const (PRD D2, ADR 0038 §Decision 1) ----

#[test]
fn snapshot_deadline_is_twenty_seconds() {
    assert_eq!(SNAPSHOT_DEADLINE, Duration::from_secs(20));
}

#[test]
fn snapshot_deadline_exceeds_ib_snapshot_tail_and_is_not_take_first() {
    // IB: tickSnapshotEnd is expected ~11s after the request; a smaller bound false-times-out.
    assert!(SNAPSHOT_DEADLINE > Duration::from_secs(11));
    assert_ne!(SNAPSHOT_DEADLINE, TAKE_FIRST_TIMEOUT);
}

// ---- the pure timeout-error builder (PRD D1/D4, ADR 0038 §Decision 3) ----

#[test]
fn quote_timeout_error_contract() {
    let err = snapshot_timeout_error("NVDA", "live", "quote/NVDA");
    assert_eq!(err.code(), "timeout");
    assert_eq!(err.exit_code(), 6);
    assert_eq!(err.context.as_deref(), Some("quote/NVDA"));
    let msg = err.message.as_str();
    assert!(msg.contains("NVDA"), "message must name the instrument: {msg}");
    assert!(msg.contains("20s"), "message must state the deadline: {msg}");
    assert!(msg.contains("md-type=live"), "message must state the md-type: {msg}");
    assert!(msg.contains("--md-type delayed"), "message must carry the delayed hint: {msg}");
    assert!(!msg.contains('\n'), "message must be single-line: {msg}");
}

#[test]
fn option_quote_timeout_error_names_full_contract_label() {
    let err = snapshot_timeout_error("NVDA 20261002 225 C", "delayed", "option-quote");
    assert_eq!(err.code(), "timeout");
    assert_eq!(err.exit_code(), 6);
    assert_eq!(err.context.as_deref(), Some("option-quote"));
    assert!(err.message.contains("NVDA 20261002 225 C"));
    assert!(err.message.contains("md-type=delayed"));
}

// ---- regressions: connect-phase contract + unchanged CLI surface ----

fn stderr_code(args: &[&str]) -> Value {
    let output = omi().args(args).assert().failure().get_output().clone();
    let stderr = String::from_utf8_lossy(&output.stderr);
    let v: Value =
        serde_json::from_str(stderr.trim()).expect("stderr must be a JSON error envelope");
    v["error"]["code"].clone()
}

#[test]
fn quote_dead_port_is_still_a_connection_error() {
    let code = stderr_code(&[
        "--format", "json", "quote", "AAPL", "--host", "127.0.0.1", "--port", "65000",
    ]);
    assert_eq!(code, "connection");
}

#[test]
fn option_quote_dead_port_is_still_a_connection_error() {
    let code = stderr_code(&[
        "--format", "json", "option-quote", "--symbol", "AAPL", "--expiry", "20261016",
        "--strike", "250", "--right", "C", "--host", "127.0.0.1", "--port", "65000",
    ]);
    assert_eq!(code, "connection");
}

#[test]
fn snapshot_commands_gain_no_timeout_flag() {
    for cmd in ["quote", "option-quote"] {
        omi()
            .args([cmd, "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("--timeout").not())
            .stdout(predicate::str::contains("--deadline").not());
    }
}
