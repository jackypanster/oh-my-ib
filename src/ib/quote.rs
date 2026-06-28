//! `quote` — one-shot snapshot quote for a symbol. (card 02)

use ibapi::market_data::MarketDataType;
use ibapi::prelude::{Contract, TickTypes};
use serde_json::json;

use crate::cli::QuoteArgs;
use crate::config::{Config, MdType};
use crate::error::AppError;

pub fn quote(cfg: &Config, args: &QuoteArgs) -> Result<serde_json::Value, AppError> {
    let client = super::connect(cfg)?;

    let md = match &args.md_type {
        Some(s) => MdType::parse(s)?,
        None => cfg.md_type,
    };
    let (market_data_type, delayed) = match md {
        MdType::Live => (MarketDataType::Realtime, false),
        MdType::Delayed => (MarketDataType::Delayed, true),
        MdType::Frozen => (MarketDataType::Frozen, false),
    };
    client
        .switch_market_data_type(market_data_type)
        .map_err(|e| AppError::data(format!("switch_market_data_type failed: {e}"), "quote"))?;

    let contract = Contract::stock(args.symbol.as_str()).build();
    let subscription = client
        .market_data(&contract)
        .snapshot()
        .subscribe()
        .map_err(|e| AppError::data(format!("market_data failed: {e}"), "quote"))?;

    let mut ticks = serde_json::Map::new();
    for tick in subscription.iter_data() {
        let tick = tick.map_err(|e| AppError::data(format!("market_data stream: {e}"), "quote"))?;
        match tick {
            TickTypes::Price(p) => {
                ticks.insert(format!("{:?}", p.tick_type), json!(p.price));
            }
            TickTypes::Size(s) => {
                ticks.insert(format!("{:?}", s.tick_type), json!(s.size));
            }
            TickTypes::SnapshotEnd => break,
            _ => {}
        }
    }

    Ok(json!({
        "symbol": args.symbol,
        "delayed": delayed,
        "ticks": ticks,
    }))
}
