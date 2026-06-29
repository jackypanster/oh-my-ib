//! FROZEN SPEC — quote-drop-volume (review-05 follow-up C). Offline: `quote` keeps only price ticks;
//! size/volume ticks (whose gateway value is unreliable, e.g. 1.4e13) are dropped. The live "no volume
//! key in output" behavior is verified by live acceptance; this freezes the pure classifier.
//! The coder must NOT edit this file. RED until impl adds + re-exports `quote_price_tick`
//! (and adds `ibapi` to dev-dependencies).

use ibapi::contracts::tick_types::TickType;
use ibapi::market_data::realtime::{TickAttribute, TickPrice, TickSize};
use ibapi::prelude::TickTypes;
use oh_my_ib::ib::quote_price_tick;

#[test]
fn price_tick_is_kept() {
    let tick = TickTypes::Price(TickPrice {
        tick_type: TickType::DelayedClose,
        price: 283.78,
        attributes: TickAttribute::default(),
    });
    let kept = quote_price_tick(&tick);
    assert!(kept.is_some(), "price ticks must be kept");
    let (label, price) = kept.unwrap();
    assert!(label.contains("DelayedClose"), "label was {label}");
    assert_eq!(price, 283.78);
}

#[test]
fn volume_size_tick_is_dropped() {
    let tick = TickTypes::Size(TickSize {
        tick_type: TickType::DelayedVolume,
        size: 13_986_886_088_824.0,
    });
    assert!(
        quote_price_tick(&tick).is_none(),
        "size/volume ticks must be dropped (gateway volume is unreliable)"
    );
}
