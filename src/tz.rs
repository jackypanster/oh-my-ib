//! Built-in timezone-alias registration.
//!
//! Some gateways report a non-IANA timezone abbreviation that rust-ibapi rejects at the
//! connection handshake — notably the Tiger Brokers / Hong Kong gateway, which reports "HKT".
//! We register a curated set of UNAMBIGUOUS aliases before connecting so `omi` works without
//! the user setting `IBAPI_TIMEZONE_ALIASES` (that env var is read natively by ibapi and stays
//! additive / overriding).

use std::sync::Once;

/// Curated gateway-abbreviation → IANA mappings. Only UNAMBIGUOUS abbreviations are included
/// (no CST/IST/etc. that map to several zones).
pub fn builtin_aliases() -> &'static [(&'static str, &'static str)] {
    &[
        ("HKT", "Asia/Hong_Kong"),
        ("JST", "Asia/Tokyo"),
        ("KST", "Asia/Seoul"),
        ("SGT", "Asia/Singapore"),
    ]
}

/// Register the built-in aliases with ibapi. Idempotent (runs at most once per process) and
/// must be called before `Client::connect`.
pub fn register_builtin_aliases() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        for (abbr, iana) in builtin_aliases() {
            ibapi::register_timezone_alias(*abbr, *iana);
        }
    });
}
