//! `history` — historical bars for a symbol. (card 02)

use ibapi::market_data::historical::{BarSize, Duration, ToDuration, WhatToShow};
use ibapi::prelude::Contract;
use serde_json::json;

use crate::cli::HistoryArgs;
use crate::config::Config;
use crate::error::AppError;

pub fn history(cfg: &Config, args: &HistoryArgs) -> Result<serde_json::Value, AppError> {
    let client = super::connect(cfg)?;
    let contract = Contract::stock(args.symbol.as_str()).build();
    let bar_size = parse_bar(&args.bar)?;
    let duration = parse_duration(&args.duration)?;

    let data = client
        .historical_data(&contract, bar_size)
        .what_to_show(WhatToShow::Trades)
        .duration(duration)
        .fetch()
        .map_err(|e| AppError::data(format!("historical_data failed: {e}"), "history"))?;

    let bars: Vec<_> = data.bars.iter().map(|b| json!(format!("{b:?}"))).collect();
    Ok(json!({
        "symbol": args.symbol,
        "bars": bars,
    }))
}

fn parse_bar(s: &str) -> Result<BarSize, AppError> {
    let bar = match s.to_ascii_lowercase().as_str() {
        "1d" | "day" | "1day" => BarSize::Day,
        "1h" | "hour" | "1hour" => BarSize::Hour,
        "30m" | "30min" => BarSize::Min30,
        "15m" | "15min" => BarSize::Min15,
        "5m" | "5min" => BarSize::Min5,
        "1m" | "min" | "1min" => BarSize::Min,
        other => {
            return Err(AppError::config(
                format!("unsupported bar size: {other}"),
                "try one of: 1d|1h|30m|15m|5m|1m",
            ))
        }
    };
    Ok(bar)
}

fn parse_duration(s: &str) -> Result<Duration, AppError> {
    let s = s.trim();
    if s.len() < 2 {
        return Err(AppError::config(
            format!("bad duration: {s}"),
            "expected <number><unit>, e.g. 30D, 2W, 6M, 1Y",
        ));
    }
    let (num, unit) = s.split_at(s.len() - 1);
    let n: i32 = num.parse().map_err(|_| {
        AppError::config(
            format!("bad duration number: {num}"),
            "expected <number><unit>",
        )
    })?;
    let duration = match unit.to_ascii_uppercase().as_str() {
        "S" => n.seconds(),
        "D" => n.days(),
        "W" => n.weeks(),
        "M" => n.months(),
        "Y" => n.years(),
        other => {
            return Err(AppError::config(
                format!("bad duration unit: {other}"),
                "expected S|D|W|M|Y",
            ))
        }
    };
    Ok(duration)
}
