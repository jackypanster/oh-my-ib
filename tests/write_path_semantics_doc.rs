//! FROZEN SPEC — write-path-semantics card 01 (ADR 0025). Offline. The coder must NOT edit this file.
//!
//! Freezes the STRUCTURAL contract of the reference-behavior audit doc
//! `docs/write-path-semantics.md` — the deliverable that maps every field the write path sends to the
//! Tiger/IB gateway to its reference semantics + a verification tier (✅ paper-probe / 📖 doc-cite /
//! ⚠️ UNVERIFIED). Four guards (ADR 0025 §3):
//!   (a) the doc exists and carries the three tiers + a `Risk register` section;
//!   (b) every required field/semantic has a row;
//!   (c) ANTI-ROT: every field a builder sets (differs from ibapi `Order::default()`) is documented —
//!       reflection-free via serde (Order derives Serialize with plain field names, no rename). This
//!       REFINES arch.md's advisory "source-scan of trade.rs": diffing the builders' serialized OUTPUT
//!       is robust where a source regex is fragile, and catches the same class (a new builder-set field
//!       that no one documented). build_combo_order sets the identical Order field set, so scanning
//!       stk+option covers the builder surface.
//!   (d) DEFAULT-CANARY: the load-bearing ibapi `Order::default()` values the doc CLAIMS — GREEN today,
//!       fires if an upstream bump silently flips one (transmit -> false ⇒ orders staged, never sent).
//!
//! (a)/(b)/(c) are RED until the doc is written; (d) is a standing regression canary.
//!
//! NOT frozen (review-by-reading + the deferred ⚠️ paper probes, ADR 0025 §4): the SEMANTIC TRUTH of
//! each row's reference-semantics / boundary columns, and that every ⚠️ row carries a runnable probe
//! recipe. Post-build gateway-path mutations (e.g. `Order.account` stamped in place_with_client) are
//! not builder output → covered by the required-field list (b) + review, like `account` here.

use ibapi::contracts::OptionRight;
use ibapi::orders::{Action, Order, OrderOrigin, TimeInForce};
use oh_my_ib::ib::{build_option_order, build_stk_order};

const DOC: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/docs/write-path-semantics.md");

fn doc() -> String {
    std::fs::read_to_string(DOC).expect(
        "docs/write-path-semantics.md must exist — the write-path reference-behavior audit (ADR 0025)",
    )
}

/// (a) the doc exists, is substantive, and carries the audit structure (three tiers + risk register).
#[test]
fn doc_exists_and_has_the_audit_structure() {
    let text = doc();
    assert!(text.len() > 1200, "doc must be substantive (got {} bytes)", text.len());
    for marker in ["✅", "📖", "⚠️", "Risk register"] {
        assert!(
            text.contains(marker),
            "doc must contain {marker:?} (the three verification tiers + the ⚠️ risk register)"
        );
    }
}

/// (b) every required field / semantic the write path sends has a row.
#[test]
fn doc_covers_every_required_field() {
    let text = doc();
    for field in [
        // explicitly-set Order fields (build_stk/option/combo_order + stamp)
        "action", "total_quantity", "order_type", "tif", "limit_price", "account",
        // load-bearing ibapi Default fields (silently sent via ..Default::default())
        "transmit", "outside_rth", "display_size", "what_if", "origin", "exempt_code",
        // Contract surface (Contract::stock/call/put/spread defaults)
        "exchange", "currency", "multiplier", "symbol", "strike", "right", "security_type",
        // combo net-limit sign semantic (ADR 0021: negative = credit)
        "credit",
    ] {
        assert!(text.contains(field), "docs/write-path-semantics.md must document {field:?}");
    }
}

/// (c) ANTI-ROT — every field a builder sets differently from ibapi `Order::default()` must be a row.
#[test]
fn doc_documents_every_field_the_builders_set() {
    let text = doc();
    let default = serde_json::to_value(Order::default()).unwrap();
    let default = default.as_object().unwrap().clone();

    // MKT (limit None) + LMT (limit Some) stk shapes + an option order.
    let orders = [
        build_stk_order("AAPL", Action::Buy, 1.0, None).1,
        build_stk_order("AAPL", Action::Buy, 1.0, Some(5.0)).1,
        build_option_order(
            "AAPL", (2026, 9, 18), 240.0, OptionRight::Call, None, "SMART", "USD", Action::Buy, 1.0, 3.0,
        )
        .1,
    ];
    for order in orders {
        let v = serde_json::to_value(&order).unwrap();
        for (key, val) in v.as_object().unwrap() {
            if default.get(key) != Some(val) {
                assert!(
                    text.contains(key.as_str()),
                    "a builder sets {key:?} (differs from ibapi Order::default()) but \
                     docs/write-path-semantics.md has no row for it — document its gateway semantics \
                     or the write path silently diverges from the reference"
                );
            }
        }
    }
}

/// (d) DEFAULT-CANARY — pin the load-bearing ibapi `Order::default()` values the doc claims.
/// GREEN today; RED if an ibapi upgrade silently changes one of these load-bearing defaults.
#[test]
fn ibapi_default_load_bearing_values_are_pinned() {
    let d = Order::default();
    assert!(d.transmit, "transmit MUST default true — false stages the order at TWS WITHOUT sending it");
    assert!(!d.outside_rth, "outside_rth load-bearing default changed");
    assert!(!d.what_if, "what_if default changed — true makes the order a margin PREVIEW, not a real order");
    assert!(matches!(d.tif, TimeInForce::Day), "tif load-bearing default changed");
    assert_eq!(d.display_size, Some(0), "display_size load-bearing default changed");
    assert_eq!(d.origin, OrderOrigin::Customer, "origin load-bearing default changed");
    assert_eq!(d.exempt_code, -1, "exempt_code load-bearing default changed");
}
