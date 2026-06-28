//! FROZEN SPEC — tz-aliases (Phase 1.1). Offline-deterministic: asserts the built-in
//! timezone-alias table that `omi` registers before connecting. The live behavior (handshake
//! succeeds without IBAPI_TIMEZONE_ALIASES) is verified by live acceptance, not here.
//! The coder must NOT edit this file (freeze gate). RED until impl adds the `tz` module.

use oh_my_ib::tz;

#[test]
fn builtin_aliases_include_hkt() {
    assert!(
        tz::builtin_aliases()
            .iter()
            .any(|(abbr, iana)| *abbr == "HKT" && *iana == "Asia/Hong_Kong"),
        "built-in table must map HKT -> Asia/Hong_Kong (the Tiger/HK gateway timezone)"
    );
}

#[test]
fn builtin_aliases_are_wellformed() {
    let table = tz::builtin_aliases();
    assert!(!table.is_empty(), "alias table must not be empty");
    for (abbr, iana) in table {
        assert!(!abbr.is_empty(), "alias abbreviation must not be empty");
        assert!(!iana.is_empty(), "IANA name must not be empty");
        assert!(
            iana.contains('/'),
            "IANA name should look like Area/City, got {iana:?}"
        );
    }
    let mut keys: Vec<&str> = table.iter().map(|(abbr, _)| *abbr).collect();
    let total = keys.len();
    keys.sort_unstable();
    keys.dedup();
    assert_eq!(keys.len(), total, "alias keys must be unique");
}
