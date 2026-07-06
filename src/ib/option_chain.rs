//! `option-chain` — the discovery set for an underlying: which option contracts exist
//! (expirations × strikes, one row per exchange × trading class). READ ONLY.
//!
//! Backed by reqSecDefOptParams (`client.option_chain`). The stream terminates via
//! `SecurityDefinitionOptionParameterEnd`, which ibapi surfaces as a clean iterator end —
//! the same End-bounded drain class as `completed_orders` (ADR 0015/0016). Per ADR 0019 D1
//! the drain is timeout-wrapped with Instant-classified `None` arms: some gateway
//! builds/states never send the End marker, so the per-item window bounds the wait and the
//! command can never hang.

use ibapi::contracts::SecurityType;
use ibapi::prelude::Contract;
use serde_json::{json, Value};

use crate::cli::OptionChainArgs;
use crate::config::Config;
use crate::error::AppError;

/// Plain, ibapi-free chain row — the frozen test constructs these directly (the
/// `SearchRow`/`CompletedOrderRow` pattern), so the pure shaping seam is offline-testable
/// with no gateway. One row per (exchange, trading_class).
pub struct ChainRow {
    pub exchange: String,
    pub trading_class: String,
    pub multiplier: String,
    pub expirations: Vec<String>,
    pub strikes: Vec<f64>,
}

/// Pure, FROZEN seam: rows → `{underlying, conid, chains: [...]}`. Sorts each row's
/// expirations (lexicographic == chronological for YYYYMMDD) and strikes (`partial_cmp`)
/// ascending, then rows by (exchange, trading_class) — full determinism (PRD D7).
/// Zero rows ⇒ `chains: []` success (the gateway answered with nothing; the agent sees empty).
pub fn shape_option_chain(underlying: &str, conid: i32, mut rows: Vec<ChainRow>) -> Value {
    for r in &mut rows {
        r.expirations.sort();
        r.strikes
            .sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    }
    rows.sort_by(|a, b| {
        a.exchange
            .cmp(&b.exchange)
            .then_with(|| a.trading_class.cmp(&b.trading_class))
    });
    json!({
        "underlying": underlying,
        "conid": conid,
        "chains": rows
            .iter()
            .map(|r| {
                json!({
                    "exchange": r.exchange,
                    "trading_class": r.trading_class,
                    "multiplier": r.multiplier,
                    "expirations": r.expirations,
                    "strikes": r.strikes,
                })
            })
            .collect::<Vec<_>>(),
    })
}

/// Pure, frozen-testable seam (ADR 0028): client-side exchange filter over chain rows.
/// `exchange == ""` ⇒ passthrough (ALL rows, input order preserved); else retain rows
/// where `row.exchange == exchange` (exact-string, CASE-SENSITIVE), input order of the
/// retained subset preserved, no dedup. Applied in `option_chain` AFTER the drain and
/// BEFORE `shape_option_chain` — reqSecDefOptParams is queried unfiltered (Tiger's server
/// filter drops SMART; ADR 0028), so `--exchange` filters locally here. No match ⇒ empty
/// vec ⇒ `shape_option_chain` yields honest `chains: []`.
pub fn filter_chain_rows(rows: Vec<ChainRow>, exchange: &str) -> Vec<ChainRow> {
    if exchange.is_empty() {
        return rows;
    }
    rows.into_iter().filter(|r| r.exchange == exchange).collect()
}

/// Read-only option-chain drain: connect → resolve the underlying conid via
/// `contract_details` FIRST row (ADR 0019 D4) → reqSecDefOptParams (queried with NO
/// server-side exchange filter) → timeout-wrapped End-bounded drain → client-side
/// `filter_chain_rows` → shape. `--exchange` is a CLIENT-SIDE filter (ADR 0028): `""`
/// ⇒ all exchanges; `<EX>` ⇒ only that exchange (default `SMART` ⇒ the single
/// consolidated row). The server call is ALWAYS unfiltered — Tiger's server-side
/// exchange filter drops SMART (ADR 0028). Empty contract_details ⇒ `not_found`.
///
/// ADR 0016 drain posture (live-proven wedge): `timeout_iter_data(TAKE_FIRST_TIMEOUT)`
/// with Instant-classified `None` arms. A terminating `None` that starved the window is a
/// `timeout` error (exit 6) naming reqSecDefOptParams; an instant `None` is the stream
/// self-ending on `SecurityDefinitionOptionParameterEnd` ⇒ success.
pub fn option_chain(cfg: &Config, args: &OptionChainArgs) -> Result<Value, AppError> {
    let client = super::connect(cfg)?;

    // Resolve the underlying conid (FIRST row — ADR 0019 D4). Contract::stock uses the
    // SMART/USD builder defaults, matching the contract.rs parity.
    let underlying = Contract::stock(&args.symbol).build();
    let details = client
        .contract_details(&underlying)
        .map_err(|e| AppError::data(format!("contract_details failed: {e}"), "option-chain"))?;
    let conid = details.first().map(|d| d.contract.contract_id).ok_or_else(|| {
        AppError::not_found(
            format!("no contract found for {}", args.symbol),
            "option-chain",
        )
    })?;

    let subscription = client
        .option_chain(&args.symbol, "", SecurityType::Stock, conid)
        .map_err(|e| AppError::data(format!("option_chain failed: {e}"), "option-chain"))?;

    let mut rows = Vec::new();
    let mut items = subscription.timeout_iter_data(super::TAKE_FIRST_TIMEOUT);
    loop {
        let waited = std::time::Instant::now();
        match items.next() {
            Some(Ok(chain)) => {
                rows.push(ChainRow {
                    exchange: chain.exchange,
                    trading_class: chain.trading_class,
                    multiplier: chain.multiplier,
                    expirations: chain.expirations,
                    strikes: chain.strikes,
                });
            }
            Some(Err(e)) => {
                return Err(AppError::data(
                    format!("option chain stream: {e}"),
                    "option-chain",
                ))
            }
            None if waited.elapsed() >= super::TAKE_FIRST_TIMEOUT => {
                return Err(AppError::timeout(
                    format!(
                        "no SecurityDefinitionOptionParameterEnd within {}s — gateway did not answer reqSecDefOptParams (known gateway issue; a restart may or may not cure it)",
                        super::TAKE_FIRST_TIMEOUT.as_secs()
                    ),
                    "option-chain",
                ))
            }
            None => break, // instant None = stream self-ended on the End marker => success
        }
    }

    let rows = filter_chain_rows(rows, &args.exchange);
    Ok(shape_option_chain(&args.symbol, conid, rows))
}
