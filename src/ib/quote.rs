//! `quote` — one-shot snapshot quote for a symbol. (card 02)
//! Market-data type comes from the global `--md-type` / config (default delayed).
//!
//! Variadic since ADR 0013: `omi quote SYM1 [SYM2 …]` connects ONCE, switches market-data
//! type ONCE, then fetches each symbol's snapshot sequentially in input order on that one
//! client. The pure `shape_quotes` seam restores byte-identical N=1 output (1 ⇒ bare object,
//! 2+ ⇒ bare array) so existing agent flows are untouched.

use ibapi::client::blocking::Client;
use ibapi::market_data::MarketDataType;
use ibapi::prelude::{Contract, TickTypes};
use serde_json::{json, Value};

use crate::cli::QuoteArgs;
use crate::config::{Config, MdType};
use crate::error::AppError;

pub fn quote(cfg: &Config, args: &QuoteArgs) -> Result<Value, AppError> {
    // STK guard unchanged — rejected before connecting (Phase 1 supports STK only).
    if !args.sec_type.eq_ignore_ascii_case("STK") {
        return Err(AppError::config(
            format!("unsupported sec-type: {}", args.sec_type),
            "Phase 1 supports --sec-type STK only",
        ));
    }

    let client = super::connect(cfg)?;

    // md-type switch is a connection-level shared request (ibapi sync.rs:176-182): called
    // ONCE before the loop, applies to every subsequent snapshot on this client. `delayed` is
    // computed once and threaded into every row.
    let (market_data_type, delayed) = match cfg.md_type {
        MdType::Live => (MarketDataType::Realtime, false),
        MdType::Delayed => (MarketDataType::Delayed, true),
        MdType::Frozen => (MarketDataType::Frozen, false),
    };
    client
        .switch_market_data_type(market_data_type)
        .map_err(|e| AppError::data(format!("switch_market_data_type failed: {e}"), "quote"))?;

    // Fetch each symbol in input order on the one connection; fail-fast `?` on the first
    // error (operator D3 — no partial output). Consume-to-`SnapshotEnd`-then-drop per symbol
    // keeps at most ONE market-data line open at a time (request-id isolation + no pacing
    // exposure; ADR 0013). Deliberately NOT wrapped in ADR 0012's take-first timeout — these
    // drains are `SnapshotEnd`-bounded, a different stream class.
    let mut rows = Vec::with_capacity(args.symbols.len());
    for symbol in &args.symbols {
        rows.push(quote_one(&client, symbol, &args.exchange, &args.currency, delayed)?);
    }

    Ok(shape_quotes(rows))
}

/// One symbol's snapshot on an already-connected client — returns EXACTLY the pre-variadic
/// single-symbol object `{symbol, delayed, ticks{…}}` (the byte-identity red line). Error
/// contexts name the symbol (`quote/<symbol>`) so a batch failure points at the offending
/// symbol; codes/messages are unchanged (ADR 0013 records this failure-path-only context delta).
pub(crate) fn quote_one(
    client: &Client,
    symbol: &str,
    exchange: &str,
    currency: &str,
    delayed: bool,
) -> Result<Value, AppError> {
    let contract = Contract::stock(symbol)
        .on_exchange(exchange)
        .in_currency(currency)
        .build();
    let subscription = client
        .market_data(&contract)
        .snapshot()
        .subscribe()
        .map_err(|e| AppError::data(format!("market_data failed: {e}"), format!("quote/{symbol}")))?;

    let mut ticks = serde_json::Map::new();
    for tick in subscription.iter_data() {
        let tick =
            tick.map_err(|e| AppError::data(format!("market_data stream: {e}"), format!("quote/{symbol}")))?;
        if matches!(tick, TickTypes::SnapshotEnd) {
            break;
        }
        if let Some((label, price)) = quote_price_tick(&tick) {
            ticks.insert(label, json!(price));
        }
    }

    Ok(json!({
        "symbol": symbol,
        "delayed": delayed,
        "ticks": ticks,
    }))
}

/// The pure, FROZEN N-shaping seam (ADR 0013): 1 row ⇒ the bare object (byte-identical
/// pass-through — the red line), 2+ rows ⇒ the bare array in given order. Empty ⇒ `[]`
/// (defensive; unreachable via clap `required = true`).
pub fn shape_quotes(mut rows: Vec<Value>) -> Value {
    if rows.len() == 1 {
        rows.pop().expect("length checked")
    } else {
        Value::Array(rows)
    }
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
