//! reprice (ADR 0040). Offline.
//!
//! Preview never plans a submit. A confirmed modify keeps the existing order id
//! and changes only the limit. Live notional over `OMI_MAX_NOTIONAL` is exit 64
//! (`guard`) and does not submit. An ack timeout after send is exit 8
//! (`unconfirmed`) and the message forbids automatic retry. A broker rejection
//! notice is exit 7. A precondition (not LMT) is exit 4 and does not submit.
//!
//! The gateway call itself (`place_order` with that same id) is review-by-reading
//! plus paper acceptance — Tiger has not been observed. No cancel+replace.

use assert_cmd::Command;
use ibapi::orders::{Action, Order};
use ibapi::Notice;
use serde_json::{json, Value};

use oh_my_ib::ib::submit_timeout_error;
use oh_my_ib::ib::{
    build_stk_order, classify_submit_error, plan_reprice, reprice_order, RepricePlan, RepriceTarget,
};

fn omi() -> Command {
    Command::cargo_bin("omi").expect("the `omi` binary should build")
}

fn stk(qty: f64, limit: f64, multiplier: f64) -> RepriceTarget {
    RepriceTarget {
        order_id: 42,
        symbol: "AAPL".into(),
        sec_type: if multiplier == 100.0 {
            "OPT".into()
        } else {
            "STK".into()
        },
        action: "Buy".into(),
        quantity: qty,
        order_type: "LMT".into(),
        limit_price: Some(limit),
        tif: "Day".into(),
        account: "ACCT".into(),
        currency: "USD".into(),
        multiplier,
    }
}

fn notice(code: i32, message: &str) -> ibapi::Error {
    ibapi::Error::Notice(Notice {
        code,
        message: message.to_string(),
        error_time: None,
        advanced_order_reject_json: String::new(),
    })
}

fn stderr_code(cmd: &mut Command) -> (i32, Value) {
    let output = cmd.assert().failure().get_output().clone();
    let code = output.status.code().expect("exit code");
    let stderr = String::from_utf8_lossy(&output.stderr);
    let v: Value =
        serde_json::from_str(stderr.trim()).expect("stderr must be a JSON error envelope");
    (code, v)
}

// ---- preview: nothing is submitted ------------------------------------------

#[test]
fn preview_prints_current_order_new_price_and_notional_and_does_not_submit() {
    let plan = plan_reprice(false, false, 500.0, &stk(10.0, 10.0, 1.0), 11.0).unwrap();
    match plan {
        RepricePlan::Preview(v) => {
            assert_eq!(v["preview"], json!(true));
            assert_eq!(v["transmits"], json!(false), "preview must not transmit");
            assert_eq!(v["order_id"], json!(42));
            assert_eq!(v["current_order"]["limit_price"], json!(10.0));
            assert_eq!(v["current_order"]["quantity"], json!(10.0));
            assert_eq!(v["current_order"]["symbol"], json!("AAPL"));
            assert_eq!(v["new_price"], json!(11.0));
            // 10 shares × 11 × multiplier 1.
            assert_eq!(v["notional"], json!(110.0));
            assert_eq!(v["risk_blocked"], json!(false));
        }
        RepricePlan::Submit(_) => panic!("preview must not submit"),
    }
}

#[test]
fn preview_of_an_over_cap_live_order_still_sends_nothing() {
    // 10 × 100 = 1000 > 500. Preview reports the block and does not submit.
    let plan = plan_reprice(false, true, 500.0, &stk(10.0, 50.0, 1.0), 100.0).unwrap();
    match plan {
        RepricePlan::Preview(v) => {
            assert_eq!(v["transmits"], json!(false));
            assert_eq!(v["notional"], json!(1000.0));
            assert_eq!(v["risk_blocked"], json!(true));
        }
        RepricePlan::Submit(_) => panic!("over-cap preview must not submit"),
    }
}

#[test]
fn option_preview_notional_uses_multiplier_100() {
    // 2 contracts × 3.00 × 100 = 600.
    let plan = plan_reprice(false, false, 500.0, &stk(2.0, 1.0, 100.0), 3.0).unwrap();
    match plan {
        RepricePlan::Preview(v) => assert_eq!(v["notional"], json!(600.0)),
        RepricePlan::Submit(_) => panic!("preview must not submit"),
    }
}

// ---- confirmed modify: same order id, limit only ----------------------------

#[test]
fn confirmed_modify_keeps_the_same_order_id_and_replaces_only_the_limit() {
    let mut order: Order = build_stk_order("AAPL", Action::Buy, 10.0, Some(10.0)).1;
    order.order_id = 42;
    order.tif = ibapi::orders::TimeInForce::Day;
    let before = order.clone();

    let plan = plan_reprice(true, false, 500.0, &stk(10.0, 10.0, 1.0), 11.5).unwrap();
    match plan {
        RepricePlan::Submit(s) => {
            assert_eq!(s.order_id, 42, "place_order must reuse the open order id");
            assert_eq!(s.new_price, 11.5);
            let modified = reprice_order(&order, s.order_id, s.new_price);
            assert_eq!(modified.order_id, before.order_id, "not a new order id");
            assert_eq!(modified.limit_price, Some(11.5));
            assert_eq!(modified.total_quantity, before.total_quantity);
            assert_eq!(modified.action, before.action);
            assert_eq!(modified.tif, before.tif);
            assert_eq!(modified.order_type, "LMT");
            assert!(modified.transmit, "a confirmed modify must transmit");
        }
        RepricePlan::Preview(_) => panic!("--confirm must submit when the cap allows it"),
    }
}

#[test]
fn paper_is_exempt_from_the_notional_cap() {
    let plan = plan_reprice(true, false, 500.0, &stk(10.0, 50.0, 1.0), 100.0).unwrap();
    assert!(
        matches!(plan, RepricePlan::Submit(s) if s.order_id == 42 && s.new_price == 100.0),
        "paper confirm over the live cap still submits"
    );
}

// ---- risk cap: exit 64, nothing submitted -----------------------------------

#[test]
fn live_risk_cap_blocks_with_exit_64_and_does_not_submit() {
    // 10 × 100 × 1 = 1000 > 500.
    let err = plan_reprice(true, true, 500.0, &stk(10.0, 50.0, 1.0), 100.0).unwrap_err();
    assert_eq!(err.code(), "guard");
    assert_eq!(err.exit_code(), 64);
    assert!(err.to_string().contains("exceeds cap"));
}

#[test]
fn live_option_notional_uses_multiplier_against_the_cap() {
    // 1 × 6 × 100 = 600 > 500.
    let err = plan_reprice(true, true, 500.0, &stk(1.0, 1.0, 100.0), 6.0).unwrap_err();
    assert_eq!(err.exit_code(), 64);
    assert_eq!(err.code(), "guard");
}

#[test]
fn cli_invalid_notional_cap_is_exit_64_before_connect() {
    let (code, v) = stderr_code(
        omi()
            .env_remove("OMI_ALLOW_LIVE")
            .env("OMI_MAX_NOTIONAL", "nope")
            .args(["--format", "json", "reprice", "--price", "1", "7"]),
    );
    assert_eq!(code, 64, "a bad OMI_MAX_NOTIONAL is a guard block");
    assert_eq!(v["error"]["code"], "guard");
}

// ---- unconfirmed / timeout after send: exit 8, never auto-retry -------------

#[test]
fn submit_timeout_is_exit_8_and_forbids_automatic_retry() {
    let err = submit_timeout_error(42, "reprice");
    assert_eq!(err.code(), "unconfirmed");
    assert_eq!(err.exit_code(), 8);
    let msg = err.to_string();
    assert!(msg.contains("UNCONFIRMED"), "{msg}");
    assert!(msg.contains("Do NOT retry"), "{msg}");
    assert!(msg.contains("42"), "the order id must be named");
}

#[test]
fn transport_error_after_send_is_unconfirmed_not_a_rejection() {
    let err = classify_submit_error(
        &ibapi::Error::Io(std::io::Error::new(
            std::io::ErrorKind::ConnectionReset,
            "reset",
        )),
        42,
        "buy",
        true,
    );
    assert_eq!(err.code(), "unconfirmed");
    assert_eq!(err.exit_code(), 8);
    assert!(err.to_string().contains("Do NOT retry"));
}

// ---- broker rejected: exit 7. Precondition before send: exit 4 --------------

#[test]
fn broker_rejection_notice_is_exit_7() {
    let err = classify_submit_error(
        &notice(201, "Order rejected - reason:"),
        42,
        "reprice",
        true,
    );
    assert_eq!(err.code(), "rejected");
    assert_eq!(err.exit_code(), 7);
}

#[test]
fn cancel_confirmation_202_is_not_a_broker_rejection() {
    let err = classify_submit_error(&notice(202, "Order Canceled"), 42, "cancel", true);
    assert_eq!(
        err.exit_code(),
        8,
        "code 202 is a confirmation, not a reject"
    );
    assert_eq!(err.code(), "unconfirmed");
}

#[test]
fn local_validation_before_send_is_data_exit_4() {
    let err = classify_submit_error(
        &ibapi::Error::InvalidArgument("scale order".into()),
        42,
        "buy",
        false,
    );
    assert_eq!(err.code(), "data");
    assert_eq!(err.exit_code(), 4);
    assert!(err.to_string().contains("not sent"));
}

#[test]
fn non_lmt_is_a_precondition_exit_4_and_does_not_submit() {
    let mut target = stk(10.0, 10.0, 1.0);
    target.order_type = "MKT".into();
    target.limit_price = None;
    let err = plan_reprice(true, false, 500.0, &target, 11.0).unwrap_err();
    assert_eq!(err.code(), "data");
    assert_eq!(err.exit_code(), 4);
    assert!(err.to_string().contains("nothing sent"));
}

#[test]
fn combo_is_out_of_scope_exit_4() {
    let mut target = stk(1.0, 0.5, 100.0);
    target.sec_type = "BAG".into();
    let err = plan_reprice(true, false, 500.0, &target, 0.4).unwrap_err();
    assert_eq!(err.exit_code(), 4);
    assert_eq!(err.code(), "data");
}

// ---- CLI usage + the unchanged live-port gate -------------------------------

#[test]
fn cli_bad_price_is_usage_exit_64() {
    let (code, v) = stderr_code(
        omi()
            .env_remove("OMI_ALLOW_LIVE")
            .env_remove("OMI_MAX_NOTIONAL")
            .args(["--format", "json", "reprice", "--price", "0", "7"]),
    );
    assert_eq!(code, 64);
    assert_eq!(v["error"]["code"], "usage");
}

#[test]
fn cli_missing_args_is_usage_exit_64() {
    let (code, v) = stderr_code(
        omi()
            .env_remove("OMI_MAX_NOTIONAL")
            .args(["--format", "json", "reprice"]),
    );
    assert_eq!(code, 64);
    assert_eq!(v["error"]["code"], "usage");
}

#[test]
fn cli_confirm_live_without_env_is_still_the_config_gate() {
    let (code, v) = stderr_code(
        omi()
            .env_remove("OMI_ALLOW_LIVE")
            .env_remove("OMI_MAX_NOTIONAL")
            .args([
                "--format",
                "json",
                "reprice",
                "--price",
                "1",
                "7",
                "--confirm",
                "--live",
            ]),
    );
    assert_eq!(code, 5, "the live-port env gate stays config/5");
    assert_eq!(v["error"]["code"], "config");
}

#[test]
fn reprice_help_names_confirm_and_price() {
    omi()
        .args(["reprice", "--help"])
        .assert()
        .success()
        .stdout(predicates::str::contains("--confirm"))
        .stdout(predicates::str::contains("--price"));
}
