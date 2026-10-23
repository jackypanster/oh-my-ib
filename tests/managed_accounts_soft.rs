//! Soft-notice classifier + injectable retry for `managed_accounts`
//! (health / resolve_account). Offline: no live gateway.

use std::cell::Cell;

use oh_my_ib::error::ErrorKind;
use oh_my_ib::ib::{is_soft_managed_accounts_notice, managed_accounts_tolerant_with};

fn notice(code: i32, message: &str) -> ibapi::Error {
    ibapi::Error::Notice(ibapi::Notice {
        code,
        message: message.to_string(),
        error_time: None,
        advanced_order_reject_json: String::new(),
    })
}

#[test]
fn client_version_advisory_2172_is_soft() {
    assert!(
        is_soft_managed_accounts_notice(2172),
        "2172 is the Tiger/IB client-version deprecation advisory (soft until 2026-12-15)"
    );
}

#[test]
fn ibapi_warning_band_is_soft() {
    for code in [2100, 2104, 2107, 2169] {
        assert!(
            is_soft_managed_accounts_notice(code),
            "code {code} is inside ibapi WARNING_CODE_RANGE and must be soft"
        );
    }
}

#[test]
fn hard_error_codes_are_not_soft() {
    for code in [0, 200, 321, 502, 1100, 2099, 2200, 10089] {
        assert!(
            !is_soft_managed_accounts_notice(code),
            "code {code} must remain a hard failure on managed_accounts"
        );
    }
}

#[test]
fn soft_twice_then_ok_collects_two_warnings() {
    let calls = Cell::new(0u32);
    let result = managed_accounts_tolerant_with(|| {
        let n = calls.get();
        calls.set(n + 1);
        match n {
            0 => Err(notice(2172, "client version advisory")),
            1 => Err(notice(2104, "Market data farm OK")),
            _ => Ok(vec!["DU_TEST".to_string()]),
        }
    });
    let (accounts, warnings) = result.expect("soft×2 then Ok must succeed");
    assert_eq!(accounts, vec!["DU_TEST".to_string()]);
    assert_eq!(warnings.len(), 2, "exactly two soft notices before Ok");
    assert!(
        warnings.iter().any(|w| w.contains("[2172]")),
        "warnings must include Display of 2172: {warnings:?}"
    );
    assert!(
        warnings.iter().any(|w| w.contains("[2104]")),
        "warnings must include Display of 2104: {warnings:?}"
    );
    assert_eq!(calls.get(), 3);
}

#[test]
fn soft_three_times_exhausts_to_err() {
    let calls = Cell::new(0u32);
    let err = managed_accounts_tolerant_with(|| {
        calls.set(calls.get() + 1);
        Err(notice(2172, "client version advisory"))
    })
    .expect_err("soft×3 must Err after attempts exhausted");
    assert_eq!(err.kind, ErrorKind::Data);
    assert!(
        err.message.contains("soft notices after 3 attempts"),
        "message should name attempt budget: {}",
        err.message
    );
    assert_eq!(calls.get(), 3);
}

#[test]
fn non_soft_notice_fails_immediately() {
    let calls = Cell::new(0u32);
    let err = managed_accounts_tolerant_with(|| {
        calls.set(calls.get() + 1);
        Err(notice(321, "rejected"))
    })
    .expect_err("non-soft Notice must fail immediately");
    assert_eq!(err.kind, ErrorKind::Data);
    assert!(
        err.message.contains("managed_accounts failed"),
        "hard path message: {}",
        err.message
    );
    assert_eq!(calls.get(), 1, "must not retry after non-soft Notice");
}

#[test]
fn health_warnings_json_key_stable() {
    let calls = Cell::new(0u32);
    let (accounts, warnings) = managed_accounts_tolerant_with(|| {
        let n = calls.get();
        calls.set(n + 1);
        match n {
            0 => Err(notice(2172, "client version advisory")),
            1 => Err(notice(2104, "Market data farm OK")),
            _ => Ok(vec!["DU_TEST".to_string()]),
        }
    })
    .expect("soft×2 then Ok");
    assert_eq!(accounts, vec!["DU_TEST".to_string()]);
    let payload = serde_json::json!({ "warnings": warnings });
    let arr = payload
        .get("warnings")
        .expect("health JSON must expose stable key `warnings`")
        .as_array()
        .expect("`warnings` must be an array");
    assert_eq!(arr.len(), 2);
}
