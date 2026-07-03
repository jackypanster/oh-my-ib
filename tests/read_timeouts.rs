//! FROZEN SPEC — read-timeouts (card 01). Offline. The coder must NOT edit this file.
//!
//! Freezes the timeout error contract (ADR 0012): `AppError::timeout` → code `"timeout"` /
//! exit 6 / Display shape, the full code⇔exit envelope table around the new variant
//! (regression), the shared `TAKE_FIRST_TIMEOUT` const (10s — PRD D3), and the unchanged CLI
//! surface (no new subcommand/flag; a dead port stays `code="connection"` — the timeout class
//! must never shadow connect-phase errors). RED until impl adds `ErrorKind::Timeout` +
//! `AppError::timeout` and exports `oh_my_ib::ib::TAKE_FIRST_TIMEOUT` (the imports below won't
//! resolve until then).
//!
//! NOT frozen (reviewed-by-reading + operator live acceptance, PRD criterion 8): the two seam
//! swaps themselves — `timeout_iter_data(TAKE_FIRST_TIMEOUT).next()` in `pnl_with_client`
//! (src/ib/pnl.rs) and `sweep_pnl_singles` (src/ib/pnl_by_position.rs), the cure-message
//! wording, the sweep arm's conid attribution, and the untouched `Some(Err)` arms. Triggering a
//! real read-timeout offline needs a fake IB server — forbidden (no-mock rule,
//! agent_docs/tests.md); arch.md §Freeze coverage records this split.

use std::time::Duration;

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;

use oh_my_ib::error::AppError;
use oh_my_ib::ib::TAKE_FIRST_TIMEOUT;

fn omi() -> Command {
    Command::cargo_bin("omi").expect("the `omi` binary should build")
}

// ---- the timeout error contract (PRD D2: machine-distinguishable "restart the gateway") ----

#[test]
fn timeout_error_has_code_timeout_and_exit_6() {
    let err = AppError::timeout(
        "no PnL reading within 10s — gateway PnL channel may be wedged; restart the gateway",
        "pnl",
    );
    assert_eq!(err.code(), "timeout");
    assert_eq!(err.exit_code(), 6);
}

#[test]
fn timeout_display_renders_code_message_context() {
    let err = AppError::timeout("no PnL reading within 10s", "brief/pnl");
    assert_eq!(
        err.to_string(),
        "[timeout] no PnL reading within 10s (brief/pnl)"
    );
}

#[test]
fn code_and_exit_table_regression_around_the_new_variant() {
    // The envelope contract, pinned: adding Timeout must not rename/renumber any sibling.
    let table: [(AppError, &str, i32); 7] = [
        (AppError::connection("m", "c"), "connection", 2),
        (AppError::not_found("m", "c"), "not_found", 3),
        (AppError::data("m", "c"), "data", 4),
        (AppError::config("m", "c"), "config", 5),
        (AppError::usage("m", "c"), "usage", 64),
        (AppError::other("m"), "error", 1),
        (AppError::timeout("m", "c"), "timeout", 6),
    ];
    for (err, code, exit) in table {
        assert_eq!(err.code(), code, "code drifted for {code}");
        assert_eq!(err.exit_code(), exit, "exit code drifted for {code}");
    }
}

// ---- the shared bound (PRD D3: ONE pub const, 10s, not configurable) ----

#[test]
fn take_first_timeout_is_the_shared_ten_second_const() {
    assert_eq!(TAKE_FIRST_TIMEOUT, Duration::from_secs(10));
}

// ---- unchanged CLI surface (PRD scope: no new CLI/config surface) ----

#[test]
fn help_gains_no_timeout_flag_or_subcommand() {
    // Verified 0 matches pre-freeze: this feature must add NO CLI surface.
    omi()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("timeout").not());
}

#[test]
fn pnl_help_gains_no_timeout_flag() {
    omi()
        .args(["pnl", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("timeout").not());
}

#[test]
fn pnl_dead_port_is_still_a_connection_error() {
    // Dead port ⇒ deterministic refusal BEFORE any read: the timeout class must never shadow
    // the connect-phase error contract (arch.md §Freeze coverage).
    let output = omi()
        .args(["--format", "json", "pnl", "--host", "127.0.0.1", "--port", "65000"])
        .assert()
        .failure()
        .get_output()
        .clone();
    let stderr = String::from_utf8_lossy(&output.stderr);
    let v: Value =
        serde_json::from_str(stderr.trim()).expect("stderr must be a JSON error envelope");
    assert_eq!(
        v["error"]["code"], "connection",
        "connect-phase failures must keep code=\"connection\""
    );
}
