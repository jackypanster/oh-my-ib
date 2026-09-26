//! `quote` — one-shot snapshot quote for a symbol. (card 02)
//! Market-data type comes from the global `--md-type` / config (default delayed).
//!
//! Variadic since ADR 0013: `omi quote SYM1 [SYM2 …]` connects ONCE, switches market-data
//! type ONCE, then fetches each symbol's snapshot sequentially in input order on that one
//! client. The pure `shape_quotes` seam restores byte-identical N=1 output (1 ⇒ bare object,
//! 2+ ⇒ bare array) so existing agent flows are untouched.

use ibapi::client::blocking::Client;
use ibapi::market_data::MarketDataType;
use ibapi::prelude::{Contract, SubscriptionItem, TickTypes};
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
    // computed once and threaded into every row; `md_label` feeds the bounded-drain timeout
    // error (ADR 0038).
    let (market_data_type, delayed, md_label) = match cfg.md_type {
        MdType::Live => (MarketDataType::Realtime, false, "live"),
        MdType::Delayed => (MarketDataType::Delayed, true, "delayed"),
        MdType::Frozen => (MarketDataType::Frozen, false, "frozen"),
    };
    client
        .switch_market_data_type(market_data_type)
        .map_err(|e| AppError::data(format!("switch_market_data_type failed: {e}"), "quote"))?;

    // Fetch each symbol in input order on the one connection; fail-fast `?` on the first
    // error (operator D3 — no partial output). Consume-to-`SnapshotEnd`-then-drop per symbol
    // keeps at most ONE market-data line open at a time (request-id isolation + no pacing
    // exposure; ADR 0013). The drain is bounded by the TOTAL `SNAPSHOT_DEADLINE` (ADR 0038;
    // live-proven silent snapshots), NOT by ADR 0012's take-first per-item window.
    let mut rows = Vec::with_capacity(args.symbols.len());
    for symbol in &args.symbols {
        rows.push(quote_one(
            &client,
            symbol,
            &args.exchange,
            &args.currency,
            delayed,
            md_label,
        )?);
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
    md_type: &str,
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

    // Total-deadline drain (ADR 0038 §Decision 2): `next_timeout(remaining)` repeatedly until
    // SnapshotEnd / Err / None, instead of a bare `iter_data()`. `Notice` is skipped for
    // `iter_data` parity (it filtered notices); `None` is Instant-classified — at/after the
    // deadline it is a timeout, before it the stream self-ended (old behavior). Dropping the
    // timed-out `subscription` sends CancelMktData (ibapi cancellation on Drop).
    let deadline = std::time::Instant::now() + super::SNAPSHOT_DEADLINE;
    let mut ticks = serde_json::Map::new();
    loop {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        match subscription.next_timeout(remaining) {
            Some(Ok(SubscriptionItem::Data(TickTypes::SnapshotEnd))) => break,
            Some(Ok(SubscriptionItem::Data(tick))) => {
                if let Some((label, price)) = quote_price_tick(&tick) {
                    ticks.insert(label, json!(price));
                }
            }
            Some(Ok(SubscriptionItem::Notice(_))) => continue,
            Some(Err(e)) => {
                return Err(AppError::data(
                    format!("market_data stream: {e}"),
                    format!("quote/{symbol}"),
                ))
            }
            None if std::time::Instant::now() >= deadline => {
                return Err(super::snapshot_timeout_error(
                    symbol,
                    md_type,
                    &format!("quote/{symbol}"),
                ))
            }
            None => break,
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
