# arch — quote-drop-volume (follow-up C)

Tiny, behavior-narrowing change confined to `src/ib/quote.rs` (+ one re-export). No new module.

## Pieces
1. `src/ib/quote.rs`:
   - `pub fn quote_price_tick(tick: &TickTypes) -> Option<(String, f64)>`
     ```
     match tick {
         TickTypes::Price(p) => Some((format!("{:?}", p.tick_type), p.price)),
         _ => None,
     }
     ```
   - Tick loop becomes: `if matches!(tick, TickTypes::SnapshotEnd) { break }` then
     `if let Some((label, price)) = quote_price_tick(&tick) { ticks.insert(label, json!(price)); }`.
     The `TickTypes::Size(_)` arm is **removed** — volume and all size ticks are dropped.
2. `src/ib/mod.rs`: `pub use quote::quote_price_tick;` (the `quote` module is private; the test imports
   `oh_my_ib::ib::quote_price_tick`).

## Frozen test (tests/quote_ticks.rs, offline)
Add `ibapi` to `[dev-dependencies]` (it is a normal dep; dev-deps are separate for integration tests).
Construct two ticks and assert the classifier — confirmed all fields are public/constructible:
```
use oh_my_ib::ib::quote_price_tick;
use ibapi::prelude::TickTypes;
use ibapi::market_data::realtime::{TickAttribute, TickPrice, TickSize};
use ibapi::contracts::tick_types::TickType;   // TickType::DelayedClose / DelayedVolume

let price = TickTypes::Price(TickPrice { tick_type: TickType::DelayedClose, price: 283.78,
                                         attributes: TickAttribute::default() });
let volume = TickTypes::Size(TickSize { tick_type: TickType::DelayedVolume, size: 1.4e13 });
assert!(quote_price_tick(&price).is_some());
assert!(quote_price_tick(&volume).is_none());
```
(If a re-export path differs, impl resolves it via cargo; the variants/fields above are confirmed in
ibapi 3.1: `TickAttribute` derives `Default`; `TickType::{DelayedClose=75, DelayedVolume=74}`.)

## Freeze coverage
Frozen: `quote_price_tick` keeps Price, drops Size (offline). NOT frozen — reviewed + live acceptance:
`omi --live quote AAPL --md-type delayed` shows price keys and no `*Volume`/size key, valid JSON.

No ADR (small, additive, behavior-narrowing, reversible).
