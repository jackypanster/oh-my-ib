//! `option-quote` — snapshot quote + best-effort greeks for one option contract. READ ONLY.
//!
//! Reuses the `quote.rs` snapshot-drain class (ADR 0013): `market_data(..).snapshot()` drained
//! to `SnapshotEnd` under the shared TOTAL `SNAPSHOT_DEADLINE` (ADR 0038; amends ADR 0019 D2's
//! "deliberately NOT timeout-wrapped" posture — live-proven silent snapshots). Price ticks flow
//! into `ticks` via `quote_price_tick`; OptionComputation rows are filtered to the MODEL rows
//! only (ADR 0019 D3): `ModelOption`(13)/`DelayedModelOption`(83) populate `greeks`
//! (last-write-wins), every other computation row is dropped. If no model row arrives before
//! `SnapshotEnd` the `greeks` key is ABSENT and the output is still a success — under
//! delayed+snapshot some data farms never push computations, and failing there would break the
//! default path.

use ibapi::contracts::tick_types::TickType;
use ibapi::market_data::MarketDataType;
use ibapi::prelude::{Contract, SubscriptionItem, TickTypes};
use serde_json::{json, Map, Value};

use crate::cli::OptionQuoteArgs;
use crate::config::{Config, MdType};
use crate::error::AppError;

/// Plain, ibapi-free greeks row. All fields optional — emitted key-by-key (omit-if-None) so
/// the frozen test constructs these directly with `..Default::default()`.
#[derive(Default)]
pub struct GreeksRow {
    pub implied_volatility: Option<f64>,
    pub delta: Option<f64>,
    pub gamma: Option<f64>,
    pub vega: Option<f64>,
    pub theta: Option<f64>,
    pub option_price: Option<f64>,
    pub underlying_price: Option<f64>,
}

/// Pure, FROZEN seam (ADR 0019 D3): `Some(GreeksRow)` ONLY for an
/// `OptionComputation` whose `field` is `ModelOption`(13) or `DelayedModelOption`(83) — the
/// rows TWS shows as "the greeks". Side computations (bid/ask/last) and custom rows are
/// per-side IVs, not the model surface, and yield `None`. Every non-computation tick (price,
/// size, etc.) yields `None` — those belong to `ticks`.
pub fn option_quote_greeks(tick: &TickTypes) -> Option<GreeksRow> {
    match tick {
        TickTypes::OptionComputation(c)
            if matches!(
                c.field,
                TickType::ModelOption | TickType::DelayedModelOption
            ) =>
        {
            Some(GreeksRow {
                implied_volatility: c.implied_volatility,
                delta: c.delta,
                gamma: c.gamma,
                vega: c.vega,
                theta: c.theta,
                option_price: c.option_price,
                underlying_price: c.underlying_price,
            })
        }
        _ => None,
    }
}

/// Pure, FROZEN seam: assemble `{contract, delayed, ticks, greeks?}`. The `contract` echo is
/// the exact 8 keys {symbol, expiry, strike, right, exchange, currency, multiplier: "100",
/// trading_class (null when absent)}; `right` is echoed normalized ("C"|"P"). The `greeks` key
/// is present IFF a model row arrived (last-write-wins); inside it only `Some`-valued fields
/// appear. `ticks` pass through unchanged.
#[allow(clippy::too_many_arguments)] // the signature IS the contract (frozen spec; brief.rs:27 precedent)
pub fn shape_option_quote(
    symbol: &str,
    expiry: &str,
    strike: f64,
    right: &str,
    exchange: &str,
    currency: &str,
    trading_class: Option<&str>,
    delayed: bool,
    ticks: Map<String, Value>,
    greeks: Option<GreeksRow>,
) -> Value {
    let normalized = normalize_right(right);
    let contract = json!({
        "symbol": symbol,
        "expiry": expiry,
        "strike": strike,
        "right": normalized,
        "exchange": exchange,
        "currency": currency,
        "multiplier": "100",
        "trading_class": trading_class,
    });
    let mut out = json!({
        "contract": contract,
        "delayed": delayed,
        "ticks": ticks,
    });
    if let Some(g) = greeks {
        let mut greeks_obj = Map::new();
        if let Some(v) = g.implied_volatility {
            greeks_obj.insert("implied_volatility".to_string(), json!(v));
        }
        if let Some(v) = g.delta {
            greeks_obj.insert("delta".to_string(), json!(v));
        }
        if let Some(v) = g.gamma {
            greeks_obj.insert("gamma".to_string(), json!(v));
        }
        if let Some(v) = g.vega {
            greeks_obj.insert("vega".to_string(), json!(v));
        }
        if let Some(v) = g.theta {
            greeks_obj.insert("theta".to_string(), json!(v));
        }
        if let Some(v) = g.option_price {
            greeks_obj.insert("option_price".to_string(), json!(v));
        }
        if let Some(v) = g.underlying_price {
            greeks_obj.insert("underlying_price".to_string(), json!(v));
        }
        if let Some(obj) = out.as_object_mut() {
            obj.insert("greeks".to_string(), Value::Object(greeks_obj));
        }
    }
    out
}

/// Normalize a right token to the canonical `"C"`/`"P"`. Accepts c/C/call/CALL/p/P/put/PUT
/// (case-insensitive). Returns `None` for anything else (caller emits a usage error).
pub(crate) fn normalize_right(right: &str) -> Option<&'static str> {
    match right.to_ascii_lowercase().as_str() {
        "c" | "call" => Some("C"),
        "p" | "put" => Some("P"),
        _ => None,
    }
}

/// Parse an 8-digit YYYYMMDD expiry into (year, month, day) with m∈1..=12, d∈1..=31.
/// Returns `None` for any non-conforming shape (validation precedes connection).
pub(crate) fn parse_expiry(expiry: &str) -> Option<(u16, u8, u8)> {
    let bytes = expiry.as_bytes();
    if bytes.len() != 8 || !bytes.iter().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let year: u16 = expiry[0..4].parse().ok()?;
    let month: u8 = expiry[4..6].parse().ok()?;
    let day: u8 = expiry[6..8].parse().ok()?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    Some((year, month, day))
}

/// Read-only option snapshot + best-effort greeks: pre-connect validation → connect → md-type
/// switch → OptionBuilder contract → bare SnapshotEnd drain. Error context `"option-quote"`.
pub fn option_quote(cfg: &Config, args: &OptionQuoteArgs) -> Result<Value, AppError> {
    // Pre-connect validation (offline-frozen, usage envelope): right / strike / expiry shape.
    // Ordering matters — frozen tests assert usage precedes any connection attempt.
    let normalized = normalize_right(&args.right).ok_or_else(|| {
        AppError::usage(
            format!("invalid --right {}: expected C|CALL or P|PUT", args.right),
            "option-quote",
        )
    })?;
    // Finite-positive: clap's f64 parser accepts NaN/inf, which are not valid option strikes
    // and must be rejected here (before connect) — NaN also fails `> 0.0`, but `inf` passes it.
    if !args.strike.is_finite() || args.strike <= 0.0 {
        return Err(AppError::usage(
            format!("--strike must be a finite positive number (got {})", args.strike),
            "option-quote",
        ));
    }
    let (year, month, day) = parse_expiry(&args.expiry).ok_or_else(|| {
        AppError::usage(
            format!(
                "invalid --expiry {}: expected 8-digit YYYYMMDD with month 1-12 and day 1-31",
                args.expiry
            ),
            "option-quote",
        )
    })?;

    let client = super::connect(cfg)?;

    // md-type switch (quote.rs:32-39 verbatim) — `delayed` threads into the output echo;
    // `md_label` feeds the bounded-drain timeout error (ADR 0038).
    let (market_data_type, delayed, md_label) = match cfg.md_type {
        MdType::Live => (MarketDataType::Realtime, false, "live"),
        MdType::Delayed => (MarketDataType::Delayed, true, "delayed"),
        MdType::Frozen => (MarketDataType::Frozen, false, "frozen"),
    };
    client
        .switch_market_data_type(market_data_type)
        .map_err(|e| AppError::data(format!("switch_market_data_type failed: {e}"), "option-quote"))?;

    // OptionBuilder contract (multiplier defaults to 100; exchange SMART; currency USD).
    let mut builder = match normalized {
        "C" => Contract::call(&args.symbol),
        _ => Contract::put(&args.symbol),
    }
    .strike(args.strike)
    .expires_on(year, month, day)
    .on_exchange(&args.exchange)
    .in_currency(&args.currency);
    if let Some(tc) = &args.trading_class {
        builder = builder.trading_class(tc);
    }
    let contract = builder.build();

    let subscription = client
        .market_data(&contract)
        .snapshot()
        .subscribe()
        .map_err(|e| AppError::data(format!("market_data failed: {e}"), "option-quote"))?;

    // Total-deadline drain (ADR 0038 §Decision 2; quote.rs verbatim): `next_timeout(remaining)`
    // until SnapshotEnd / Err / None; `Notice` skipped (iter_data parity); `None` Instant-
    // classified — at/after the deadline a timeout, before it stream-self-end ⇒ success.
    // Dropping the timed-out `subscription` sends CancelMktData (ibapi cancellation on Drop).
    let instrument_label = format!("{} {} {} {}", args.symbol, args.expiry, args.strike, normalized);
    let deadline = std::time::Instant::now() + super::SNAPSHOT_DEADLINE;
    let mut ticks: Map<String, Value> = Map::new();
    let mut greeks: Option<GreeksRow> = None;
    loop {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        match subscription.next_timeout(remaining) {
            Some(Ok(SubscriptionItem::Data(TickTypes::SnapshotEnd))) => break,
            Some(Ok(SubscriptionItem::Data(tick))) => {
                if let Some((label, price)) = super::quote_price_tick(&tick) {
                    ticks.insert(label, json!(price));
                }
                if let Some(row) = option_quote_greeks(&tick) {
                    greeks = Some(row); // last-model-row-wins (ADR 0019 D3)
                }
            }
            Some(Ok(SubscriptionItem::Notice(_))) => continue,
            Some(Err(e)) => {
                return Err(AppError::data(
                    format!("market_data stream: {e}"),
                    "option-quote",
                ))
            }
            None if std::time::Instant::now() >= deadline => {
                return Err(super::snapshot_timeout_error(
                    &instrument_label,
                    md_label,
                    "option-quote",
                ))
            }
            None => break,
        }
    }

    Ok(shape_option_quote(
        &args.symbol,
        &args.expiry,
        args.strike,
        &args.right,
        &args.exchange,
        &args.currency,
        args.trading_class.as_deref(),
        delayed,
        ticks,
        greeks,
    ))
}
