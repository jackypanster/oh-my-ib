//! `quote` — one-shot snapshot quote for a symbol. (card 02)
//! Market-data type comes from the global `--md-type` / config (default delayed).

use ibapi::market_data::MarketDataType;
use ibapi::prelude::{Contract, TickTypes};
use serde_json::{json, Value};

use crate::cli::QuoteArgs;
use crate::config::{Config, MdType};
use crate::error::AppError;

pub fn quote(cfg: &Config, args: &QuoteArgs) -> Result<Value, AppError> {
    if !args.sec_type.eq_ignore_ascii_case("STK") {
        return Err(AppError::config(
            format!("unsupported sec-type: {}", args.sec_type),
            "Phase 1 supports --sec-type STK only",
        ));
    }

    let client = super::connect(cfg)?;

    let (market_data_type, delayed) = match cfg.md_type {
        MdType::Live => (MarketDataType::Realtime, false),
        MdType::Delayed => (MarketDataType::Delayed, true),
        MdType::Frozen => (MarketDataType::Frozen, false),
    };
    client
        .switch_market_data_type(market_data_type)
        .map_err(|e| AppError::data(format!("switch_market_data_type failed: {e}"), "quote"))?;

    let contract = Contract::stock(args.symbol.as_str())
        .on_exchange(args.exchange.as_str())
        .in_currency(args.currency.as_str())
        .build();
    let subscription = client
        .market_data(&contract)
        .snapshot()
        .subscribe()
        .map_err(|e| AppError::data(format!("market_data failed: {e}"), "quote"))?;

    let mut ticks = serde_json::Map::new();
    for tick in subscription.iter_data() {
        let tick = tick.map_err(|e| AppError::data(format!("market_data stream: {e}"), "quote"))?;
        if matches!(tick, TickTypes::SnapshotEnd) {
            break;
        }
        if let Some((label, price)) = quote_price_tick(&tick) {
            ticks.insert(label, json!(price));
        }
    }

    Ok(json!({
        "symbol": args.symbol,
        "delayed": delayed,
        "ticks": ticks,
    }))
}

/// The `(label, price)` to keep for a quote tick: `Some` only for price ticks. Size ticks — which
/// include the gateway's unreliable volume (observed at 1.4e13) — are dropped; use `omi history`
/// for volume.
pub fn quote_price_tick(tick: &TickTypes) -> Option<(String, f64)> {
    match tick {
        TickTypes::Price(p) => Some((format!("{:?}", p.tick_type), p.price)),
        _ => None,
    }
}
