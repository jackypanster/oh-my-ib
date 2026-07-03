//! `search` — fuzzy symbol/company lookup via `reqMatchingSymbols`. (card 01)
//!
//! The plain-bounded-call read class (ADR 0014): `matching_symbols` sends ONE request,
//! reads ONE `SymbolSamples` message, and returns the matches directly — no subscription
//! object reaches this code, so it is neither the drain-to-End class nor the markerless
//! take-first class (ADR 0012). Consequently: NO STK guard (search is metadata, not
//! market-data), NO account resolution, NO md-type switch, NO `TAKE_FIRST_TIMEOUT`.
//! Errors map to the existing `data` envelope with context `search`.

use serde_json::{json, Value};

use crate::cli::SearchArgs;
use crate::config::Config;
use crate::error::AppError;

/// Plain, ibapi-free match row — the frozen test constructs these directly (the
/// `PnlSingleRow` pattern), so the pure shaping seam is offline-testable with no gateway.
pub struct SearchRow {
    pub conid: i32,
    pub symbol: String,
    pub sec_type: String,
    pub primary_exchange: String,
    pub currency: String,
    /// Company name; `""` when the gateway omits it (older servers). Pass-through, NOT null —
    /// the `pnl_number` sentinel rule is for money `f64` only; nothing here is money.
    pub description: String,
    pub derivative_sec_types: Vec<String>,
}

/// The pure, FROZEN shaping seam: rows in gateway order → a JSON array of exact 7-key
/// objects (PRD D3 pass-through — no filtering, no re-ranking, strings pass through as-is,
/// `""` stays `""`). Empty ⇒ `json!([])`. Identity fields per row let the agent filter with
/// full information; a filtered-empty result would be indistinguishable from "no matches".
pub fn shape_search(rows: Vec<SearchRow>) -> Value {
    Value::Array(
        rows.into_iter()
            .map(|r| {
                json!({
                    "conid": r.conid,
                    "symbol": r.symbol,
                    "sec_type": r.sec_type,
                    "primary_exchange": r.primary_exchange,
                    "currency": r.currency,
                    "description": r.description,
                    "derivative_sec_types": r.derivative_sec_types,
                })
            })
            .collect(),
    )
}

/// Read-only fuzzy symbol/company search: connect → one `matching_symbols` call → map fields
/// → shape. Gateway order is the contract; IB rate-limits `reqMatchingSymbols` (~1/sec,
/// IB-side) — one request per invocation makes this moot.
pub fn search(cfg: &Config, args: &SearchArgs) -> Result<Value, AppError> {
    let client = super::connect(cfg)?;
    let matches = client
        .matching_symbols(&args.pattern)
        .map_err(|e| AppError::data(format!("matching_symbols failed: {e}"), "search"))?;
    let rows = matches
        .into_iter()
        .map(|d| {
            let c = d.contract;
            SearchRow {
                conid: c.contract_id,
                symbol: c.symbol.to_string(),
                sec_type: c.security_type.to_string(),
                primary_exchange: c.primary_exchange.to_string(),
                currency: c.currency.to_string(),
                description: c.description,
                derivative_sec_types: d.derivative_security_types,
            }
        })
        .collect();
    Ok(shape_search(rows))
}
