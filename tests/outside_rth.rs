//! FROZEN SPEC — outside-rth (card 01): the opt-in extended-hours flag on the STK order path
//! (ADR 0032). Offline. The coder must NOT edit this file.
//!
//! Freezes `apply_outside_rth(order: &mut Order, outside_rth: bool) -> Result<(), String>` — the
//! post-build seam that sets `Order.outside_rth`. It refuses ONLY the MKT+outside-RTH corner (a
//! market order cannot fill outside regular trading hours), returning `Err` whose message names the
//! limit requirement; every other case is `Ok` and sets the flag verbatim (`false` — today's default
//! — is always accepted, so flagless behavior stays byte-identical). Also freezes the preview echo:
//! `shape_preview` carries `outside_rth` INSIDE its `"order"` sub-object (the existing
//! `order_preview_command.rs` still owns the top-level-keys assertion and stays green). And the CLI
//! contract: `buy --outside-rth` without `--limit` ⇒ `code="config"` (exit 5, a flag-combination
//! error); with a limit it passes the guard and reaches the connection stage.
//!
//! RED until impl adds `apply_outside_rth` to `src/ib/trade.rs`, re-exports it at `src/ib/mod.rs`,
//! adds the `--outside-rth` flag to `OrderArgs`, wires it into `place()`, and echoes it in
//! `shape_preview` (the `apply_outside_rth` import below will not resolve until then).
//!
//! NOT frozen (reviewed-by-reading, ADR 0032 §Freeze-coverage): the `place()` wiring (the
//! `apply_outside_rth` call-site, its `map_err(config)` bucket, that it runs before `place_core`);
//! the `shape_preview` source edit (its effect IS frozen, via the echo asserts below);
//! `build_stk_order` / `shape_order_ack` unchanged. NOT frozen (operator acceptance): a real place
//! on `:4002` shows the flag in `omi orders`; a real post-market fill on `:4001` is entitlement-gated.

use assert_cmd::Command;
use ibapi::orders::Action;
use serde_json::{json, Value};

use oh_my_ib::ib::{apply_outside_rth, build_stk_order, shape_preview};

fn omi() -> Command {
    let mut cmd = Command::cargo_bin("omi").expect("the `omi` binary should build");
    // The gate reads the environment: start every test from a clean slate.
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

// ---- pure seam: apply_outside_rth (offline) --------------------------------

#[test]
fn lmt_order_accepts_outside_rth_true() {
    // LMT (has a limit) + the flag ⇒ Ok, and the order carries outside_rth = true.
    let mut order = build_stk_order("AAPL", Action::Buy, 1.0, Some(150.0)).1;
    assert!(apply_outside_rth(&mut order, true).is_ok(), "LMT + --outside-rth is admitted");
    assert!(order.outside_rth, "LMT + --outside-rth must set outside_rth true");
}

#[test]
fn lmt_order_outside_rth_false_is_rth_only_default() {
    // The flag absent ⇒ false ⇒ today's RTH-only default, byte-identical.
    let mut order = build_stk_order("AAPL", Action::Buy, 1.0, Some(150.0)).1;
    assert!(apply_outside_rth(&mut order, false).is_ok());
    assert!(!order.outside_rth, "no flag ⇒ RTH-only (outside_rth false)");
}

#[test]
fn mkt_order_refuses_outside_rth_true() {
    // MKT (no limit) + the flag ⇒ hard refuse; the message must name the limit requirement,
    // and a refused apply must NOT half-mutate the flag.
    let mut order = build_stk_order("MSFT", Action::Sell, 1.0, None).1;
    let err = apply_outside_rth(&mut order, true).expect_err("MKT + --outside-rth must be refused");
    assert!(err.contains("limit"), "refuse message must name the limit requirement: {err}");
    assert!(!order.outside_rth, "a refused apply must leave outside_rth false");
}

#[test]
fn mkt_order_outside_rth_false_is_unaffected() {
    // MKT without the flag is unchanged (false is always Ok, including MKT).
    let mut order = build_stk_order("MSFT", Action::Sell, 1.0, None).1;
    assert!(apply_outside_rth(&mut order, false).is_ok(), "MKT without the flag is unaffected");
    assert!(!order.outside_rth);
}

// ---- preview echo: shape_preview carries outside_rth inside "order" ---------

#[test]
fn preview_echoes_outside_rth_true() {
    let mut order = build_stk_order("AAPL", Action::Buy, 1.0, Some(150.0)).1;
    apply_outside_rth(&mut order, true).expect("LMT + flag is ok");
    let out = shape_preview(json!({"sec_type": "STK"}), &order, 1.0, "USD");
    assert_eq!(
        out["order"]["outside_rth"],
        json!(true),
        "preview must echo outside_rth inside the order object"
    );
}

#[test]
fn preview_default_order_shows_outside_rth_false() {
    // A plain built order (flag never applied) ⇒ the preview shows false, key present.
    let (_c, order) = build_stk_order("AAPL", Action::Buy, 1.0, Some(150.0));
    let out = shape_preview(json!({"sec_type": "STK"}), &order, 1.0, "USD");
    assert_eq!(out["order"]["outside_rth"], json!(false), "default order ⇒ outside_rth false");
}

// ---- CLI contract (black-box): --outside-rth requires --limit ---------------

#[test]
fn cli_outside_rth_without_limit_is_config_error() {
    // MKT (no --limit) + --outside-rth ⇒ config (exit 5), offline, before any connect.
    expect_error_code(
        omi(),
        &["--format", "json", "buy", "AAPL", "1", "--outside-rth"],
        "config",
    );
}

#[test]
fn cli_outside_rth_with_limit_passes_guard_to_connection() {
    // LMT + --outside-rth clears the guard and proceeds to connect; a dead port ⇒ connection error
    // (proves the guard PASSED — it did not refuse). Port 65000 is a throwaway dead port.
    expect_error_code(
        omi(),
        &[
            "--format", "json", "buy", "AAPL", "1", "--limit", "1", "--outside-rth", "--host",
            "127.0.0.1", "--port", "65000",
        ],
        "connection",
    );
}
