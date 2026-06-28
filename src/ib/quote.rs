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
