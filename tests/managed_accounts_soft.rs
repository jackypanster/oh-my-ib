//! Soft-notice classifier for `managed_accounts` (health / resolve_account).
//! Offline, std-only: 2172 and the ibapi warning band must be soft; hard errors must not.

use oh_my_ib::ib::is_soft_managed_accounts_notice;

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
