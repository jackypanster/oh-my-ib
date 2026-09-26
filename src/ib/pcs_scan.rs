//! `pcs-scan` — read-only put-credit-spread (PCS) ranking over a watchlist (card 01).
//!
//! Two layers, deliberately split so the freeze gate can protect the decision math without a
//! gateway (arch.md §Pure seams / ADR 0039):
//! - PURE seams (frozen by `tests/pcs_scan.rs`): params validation, spot/expiry pick, two-pass
//!   strike planning (coarse → interpolate k* → refine), leg selection + PCS math, output shape.
//! - GATEWAY flow (`pcs_scan` + `snapshot_batch`): one connection, one md-type switch, one batched
//!   stock-snapshot pass, a sequential chain step, then two batched option-snapshot passes
//!   (ADR 0039 two-pass design). Skip-not-fail (PRD D6): every per-symbol/per-contract failure
//!   becomes a `skipped` row; only usage errors and connect failures fail the whole command.
//!
//! Read-only: this module places no orders and never touches `trade.rs`.

use std::cmp::Ordering;
use std::time::{Duration, Instant};

use ibapi::client::blocking::Client;
use ibapi::contracts::SecurityType;
use ibapi::market_data::MarketDataType;
use ibapi::prelude::{Contract, Subscription, SubscriptionItem, TickTypes};
use serde_json::{json, Map, Value};

use crate::cli::PcsScanArgs;
use crate::config::{Config, MdType};
use crate::error::AppError;

use super::option_quote::{option_quote_greeks, parse_expiry, GreeksRow};
use super::quote_price_tick;

/// Max concurrent snapshot subscriptions per batch (arch/ADR 0039): 50 is below IB's default
/// 100 market-data lines, leaving headroom for other sessions on the same gateway.
pub const PCS_BATCH_LINES: usize = 50;

/// Inter-subscribe spacing inside a batch. IB paces market-data requests (~50 msgs/s), so a
/// 50-contract burst is at the edge (arch.md §Blindspot pass sanctions a ≤20ms sleep).
const SUBSCRIBE_SPACING: Duration = Duration::from_millis(20);

/// Tolerance for float-equal strike comparisons (frozen behavior: long-leg lookup in
/// `select_pcs`, listed-partner lookup in `refine_strikes`).
const STRIKE_EPSILON: f64 = 1e-6;

// ---------------------------------------------------------------------------
// Frozen pure seams
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct PcsParams {
    pub dte_min: i64,
    pub dte_max: i64,
    pub dte_target: i64,
    pub delta: f64,
    pub width: f64,
    pub min_credit_ratio: f64,
}

impl Default for PcsParams {
    fn default() -> Self {
        Self {
            dte_min: 21,
            dte_max: 45,
            dte_target: 30,
            delta: 0.20,
            width: 5.0,
            min_credit_ratio: 0.25,
        }
    }
}

/// One fetched put: the raw snapshot data the selection math works on. `bid`/`ask`/`delta`/`iv`
/// are `None` when the gateway never sent that datum (delayed snapshots often lack greeks).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PutRow {
    pub strike: f64,
    pub bid: Option<f64>,
    pub ask: Option<f64>,
    pub delta: Option<f64>,
    pub iv: Option<f64>,
}

/// Closed skip-reason set (PRD criterion 4), snake_case on the wire via [`SkipReason::as_str`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason {
    NoExpiryInWindow,
    SpotUnavailable,
    NoGreeks,
    NoLongLeg,
    BelowMinCredit,
    QuoteError,
}

impl SkipReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            SkipReason::NoExpiryInWindow => "no_expiry_in_window",
            SkipReason::SpotUnavailable => "spot_unavailable",
            SkipReason::NoGreeks => "no_greeks",
            SkipReason::NoLongLeg => "no_long_leg",
            SkipReason::BelowMinCredit => "below_min_credit",
            SkipReason::QuoteError => "quote_error",
        }
    }
}

/// Pre-connect parameter validation (usage error, exit 64). Pure and frozen so the CLI can
/// reject bad flags BEFORE any connection attempt (PRD criterion 5).
pub fn validate_params(p: &PcsParams) -> Result<(), String> {
    if p.dte_min < 0 {
        return Err(format!("--dte-min must be >= 0 (got {})", p.dte_min));
    }
    if p.dte_min > p.dte_max {
        return Err(format!(
            "--dte-min ({}) must be <= --dte-max ({})",
            p.dte_min, p.dte_max
        ));
    }
    if p.dte_target < p.dte_min || p.dte_target > p.dte_max {
        return Err(format!(
            "--dte-target ({}) must be within [--dte-min, --dte-max] ([{}, {}])",
            p.dte_target, p.dte_min, p.dte_max
        ));
    }
    if !p.delta.is_finite() || p.delta <= 0.0 || p.delta >= 1.0 {
        return Err(format!("--delta must be in (0,1) (got {})", p.delta));
    }
    if !p.width.is_finite() || p.width <= 0.0 {
        return Err(format!("--width must be finite and > 0 (got {})", p.width));
    }
    if !p.min_credit_ratio.is_finite() || p.min_credit_ratio < 0.0 || p.min_credit_ratio >= 1.0 {
        return Err(format!(
            "--min-credit-ratio must be in [0,1) (got {})",
            p.min_credit_ratio
        ));
    }
    Ok(())
}

/// `(label, price)` lookup over a snapshot tick map, tolerant of the delayed labels
/// (`DelayedLast ≡ Last`, …): the exact label wins, else its `Delayed`-prefixed twin.
fn tick_price(ticks: &Map<String, Value>, label: &str) -> Option<f64> {
    ticks
        .get(label)
        .and_then(Value::as_f64)
        .or_else(|| ticks.get(&format!("Delayed{label}")).and_then(Value::as_f64))
}

/// Spot from a stock snapshot (PRD D9): `Last>0` → mid(`Bid`,`Ask`) when both `>0` → `Close>0`
/// → `None`. A `Delayed` prefix is stripped before matching; only positive prices qualify.
pub fn pick_spot(ticks: &Map<String, Value>) -> Option<f64> {
    if let Some(last) = tick_price(ticks, "Last").filter(|v| *v > 0.0) {
        return Some(last);
    }
    if let (Some(bid), Some(ask)) = (tick_price(ticks, "Bid"), tick_price(ticks, "Ask")) {
        if bid > 0.0 && ask > 0.0 {
            return Some((bid + ask) / 2.0);
        }
    }
    tick_price(ticks, "Close").filter(|v| *v > 0.0)
}

/// Strict `YYYYMMDD` → `time::Date`; malformed dates (shape or calendar) are ignored.
fn parse_yyyymmdd(s: &str) -> Option<time::Date> {
    let bytes = s.as_bytes();
    if bytes.len() != 8 || !bytes.iter().all(u8::is_ascii_digit) {
        return None;
    }
    let year: i32 = s[0..4].parse().ok()?;
    let month: u8 = s[4..6].parse().ok()?;
    let day: u8 = s[6..8].parse().ok()?;
    let month = time::Month::try_from(month).ok()?;
    time::Date::from_calendar_date(year, month, day).ok()
}

/// Among expiries whose DTE (days from `today`) lies in `[dte_min, dte_max]`, the one nearest
/// `dte_target`; ties go to the earlier expiry. `None` ⇒ the caller skips `NoExpiryInWindow`.
pub fn pick_expiry(
    expirations: &[String],
    today: time::Date,
    p: &PcsParams,
) -> Option<(String, i64)> {
    let mut best: Option<(String, i64, i64)> = None; // (expiry, dte, |dte - target|)
    for expiry in expirations {
        let Some(date) = parse_yyyymmdd(expiry) else {
            continue;
        };
        let dte = (date - today).whole_days();
        if dte < p.dte_min || dte > p.dte_max {
            continue;
        }
        let distance = (dte - p.dte_target).abs();
        let better = match &best {
            None => true,
            Some((_, best_dte, best_distance)) => {
                distance < *best_distance || (distance == *best_distance && dte < *best_dte)
            }
        };
        if better {
            best = Some((expiry.clone(), dte, distance));
        }
    }
    best.map(|(expiry, dte, _)| (expiry, dte))
}

/// Listed strikes in `[0.65×spot, spot]`, ascending + deduped, thinned to ≤ `n` evenly spread
/// by index (both ends kept; `len ≤ n` ⇒ all). Pass 1 of ADR 0039.
pub fn coarse_strikes(strikes: &[f64], spot: f64, n: usize) -> Vec<f64> {
    let lower = 0.65 * spot;
    let mut band: Vec<f64> = strikes
        .iter()
        .copied()
        .filter(|s| *s >= lower && *s <= spot)
        .collect();
    band.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    band.dedup();
    if n == 0 || band.is_empty() {
        return Vec::new();
    }
    if band.len() <= n {
        return band;
    }
    if n == 1 {
        return vec![band[0]];
    }
    let last = band.len() - 1;
    (0..n)
        .map(|i| {
            let idx = ((i as f64) * (last as f64) / ((n - 1) as f64)).round() as usize;
            band[idx]
        })
        .collect()
}

/// Estimate the strike where |Δ| = `target` from pass-1 rows: linear interpolation between the
/// adjacent rows (ascending strike) that bracket the target; no bracket ⇒ the row whose |Δ| is
/// nearest. No rows with a delta ⇒ `None`.
pub fn estimate_target_strike(rows: &[PutRow], target: f64) -> Option<f64> {
    let mut curve: Vec<(f64, f64)> = rows
        .iter()
        .filter_map(|row| row.delta.map(|delta| (row.strike, delta.abs())))
        .collect();
    if curve.is_empty() {
        return None;
    }
    curve.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Ordering::Equal));
    for pair in curve.windows(2) {
        let (strike_a, abs_delta_a) = pair[0];
        let (strike_b, abs_delta_b) = pair[1];
        if abs_delta_a <= target && target <= abs_delta_b {
            if abs_delta_b <= abs_delta_a {
                // Equal |Δ| ⇒ the left edge (never divide by zero).
                return Some(strike_a);
            }
            return Some(
                strike_a
                    + (target - abs_delta_a) * (strike_b - strike_a) / (abs_delta_b - abs_delta_a),
            );
        }
    }
    curve
        .iter()
        .min_by(|a, b| {
            let distance_a = (a.1 - target).abs();
            let distance_b = (b.1 - target).abs();
            distance_a.partial_cmp(&distance_b).unwrap_or(Ordering::Equal)
        })
        .map(|(strike, _)| *strike)
}

/// The `n` listed strikes nearest `k*` (ties ⇒ lower), plus each one's `strike − width` partner
/// when listed (float-equal within 1e-6); ascending, deduped, minus anything in `already`.
/// Pass 2 of ADR 0039.
pub fn refine_strikes(
    strikes: &[f64],
    k_star: f64,
    width: f64,
    n: usize,
    already: &[f64],
) -> Vec<f64> {
    let mut listed: Vec<f64> = strikes.iter().copied().filter(|s| s.is_finite()).collect();
    listed.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    listed.dedup_by(|a, b| approx_eq(*a, *b));
    if listed.is_empty() || n == 0 {
        return Vec::new();
    }

    let mut nearest = listed.clone();
    nearest.sort_by(|a, b| {
        let distance_a = (a - k_star).abs();
        let distance_b = (b - k_star).abs();
        distance_a
            .partial_cmp(&distance_b)
            .unwrap_or(Ordering::Equal)
            .then_with(|| a.partial_cmp(b).unwrap_or(Ordering::Equal))
    });

    let mut out: Vec<f64> = Vec::new();
    for strike in nearest.iter().take(n) {
        out.push(*strike);
        let partner = strike - width;
        if let Some(listed_partner) = listed.iter().find(|l| approx_eq(**l, partner)) {
            out.push(*listed_partner);
        }
    }
    out.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    out.dedup_by(|a, b| approx_eq(*a, *b));
    out.retain(|strike| !already.iter().any(|a| approx_eq(*a, *strike)));
    out
}

fn approx_eq(a: f64, b: f64) -> bool {
    (a - b).abs() <= STRIKE_EPSILON
}

/// A sellable put: `bid > 0 && ask > 0` (PRD D10). Returns `(bid, ask, mid)`.
fn usable_leg(row: &PutRow) -> Option<(f64, f64, f64)> {
    match (row.bid, row.ask) {
        (Some(bid), Some(ask)) if bid > 0.0 && ask > 0.0 => Some((bid, ask, (bid + ask) / 2.0)),
        _ => None,
    }
}

/// A ranked spread candidate. All math per PRD criterion 3; `short` carries the model delta/IV,
/// `long` is the `short.strike − width` put.
#[derive(Debug, Clone, PartialEq)]
pub struct PcsCandidate {
    pub symbol: String,
    pub spot: f64,
    pub expiry: String,
    pub dte: i64,
    pub short: PutRow,
    pub long: PutRow,
    pub width: f64,
    pub credit_mid: f64,
    pub credit_natural: f64,
    pub max_loss: f64,
    pub return_on_risk: f64,
    pub breakeven: f64,
    pub pop_approx: f64,
    pub spread_pct: f64,
}

/// Select ONE PCS candidate from fetched put rows (PRD criterion 3/4 rules):
/// short = usable put with the |Δ| nearest `p.delta` (tie ⇒ lower strike) ⇒ else `NoGreeks`;
/// long = usable `short.strike − p.width` ⇒ else `NoLongLeg`;
/// `credit_mid < min_credit_ratio × width` ⇒ `BelowMinCredit`.
pub fn select_pcs(
    symbol: &str,
    spot: f64,
    expiry: &str,
    dte: i64,
    rows: &[PutRow],
    p: &PcsParams,
) -> Result<PcsCandidate, SkipReason> {
    let short = rows
        .iter()
        .filter_map(|row| {
            let (bid, ask, mid) = usable_leg(row)?;
            let abs_delta = row.delta?.abs();
            Some((row, bid, ask, mid, abs_delta))
        })
        .min_by(|a, b| {
            let distance_a = (a.4 - p.delta).abs();
            let distance_b = (b.4 - p.delta).abs();
            // Tie ⇒ lower strike. The tie test uses a tolerance because two mathematically
            // equal distances can differ by float noise (e.g. |Δ| 0.25 and 0.15 around 0.20).
            if approx_eq(distance_a, distance_b) {
                a.0.strike.partial_cmp(&b.0.strike).unwrap_or(Ordering::Equal)
            } else {
                distance_a.partial_cmp(&distance_b).unwrap_or(Ordering::Equal)
            }
        })
        .ok_or(SkipReason::NoGreeks)?;
    let (short_row, short_bid, short_ask, short_mid, short_abs_delta) = short;

    let long_target = short_row.strike - p.width;
    let (long_row, long_bid, long_ask, long_mid) = rows
        .iter()
        .filter_map(|row| {
            let (bid, ask, mid) = usable_leg(row)?;
            approx_eq(row.strike, long_target).then_some((row, bid, ask, mid))
        })
        .next()
        .ok_or(SkipReason::NoLongLeg)?;

    let credit_mid = short_mid - long_mid;
    if credit_mid < p.min_credit_ratio * p.width {
        return Err(SkipReason::BelowMinCredit);
    }

    let max_loss = (p.width - credit_mid) * 100.0;
    Ok(PcsCandidate {
        symbol: symbol.to_string(),
        spot,
        expiry: expiry.to_string(),
        dte,
        short: short_row.clone(),
        long: long_row.clone(),
        width: p.width,
        credit_mid,
        credit_natural: short_bid - long_ask,
        max_loss,
        return_on_risk: credit_mid * 100.0 / max_loss,
        breakeven: short_row.strike - credit_mid,
        pop_approx: 1.0 - short_abs_delta,
        spread_pct: ((short_ask - short_bid) / short_mid + (long_ask - long_bid) / long_mid) / 2.0,
    })
}

fn leg_json(row: &PutRow) -> Value {
    json!({
        "strike": row.strike,
        "bid": row.bid,
        "ask": row.ask,
        "mid": usable_leg(row).map(|(_, _, mid)| mid),
    })
}

fn candidate_json(candidate: &PcsCandidate) -> Value {
    let mut short = leg_json(&candidate.short);
    if let Some(short_obj) = short.as_object_mut() {
        short_obj.insert("delta".to_string(), json!(candidate.short.delta));
        short_obj.insert("iv".to_string(), json!(candidate.short.iv));
    }
    json!({
        "symbol": candidate.symbol,
        "spot": candidate.spot,
        "expiry": candidate.expiry,
        "dte": candidate.dte,
        "short": short,
        "long": leg_json(&candidate.long),
        "width": candidate.width,
        "credit_mid": candidate.credit_mid,
        "credit_natural": candidate.credit_natural,
        "max_loss": candidate.max_loss,
        "return_on_risk": candidate.return_on_risk,
        "breakeven": candidate.breakeven,
        "pop_approx": candidate.pop_approx,
        "spread_pct": candidate.spread_pct,
    })
}

/// Final output shape (PRD criterion 2): params echo, candidates sorted by `return_on_risk`
/// desc (tie ⇒ symbol asc), and `skipped` in input order. Every input symbol appears exactly
/// once — as a candidate XOR a `{symbol, reason}` skip row.
pub fn shape_pcs_scan(
    p: &PcsParams,
    mut candidates: Vec<PcsCandidate>,
    skipped: Vec<(String, SkipReason)>,
) -> Value {
    candidates.sort_by(|a, b| {
        b.return_on_risk
            .partial_cmp(&a.return_on_risk)
            .unwrap_or(Ordering::Equal)
            .then_with(|| a.symbol.cmp(&b.symbol))
    });
    json!({
        "params": {
            "dte_min": p.dte_min,
            "dte_max": p.dte_max,
            "dte_target": p.dte_target,
            "delta": p.delta,
            "width": p.width,
            "min_credit_ratio": p.min_credit_ratio,
        },
        "candidates": candidates.iter().map(candidate_json).collect::<Vec<_>>(),
        "skipped": skipped
            .iter()
            .map(|(symbol, reason)| json!({"symbol": symbol, "reason": reason.as_str()}))
            .collect::<Vec<_>>(),
    })
}

// ---------------------------------------------------------------------------
// Gateway flow (NOT frozen — review-by-reading + operator live acceptance)
// ---------------------------------------------------------------------------

/// Snapshot data for one contract: price ticks (via `quote_price_tick`) + the last model
/// greeks row (ADR 0019 D3 last-write-wins). `greeks` is `None` for stock snapshots.
struct SnapData {
    ticks: Map<String, Value>,
    greeks: Option<GreeksRow>,
}

/// A chain row already chosen for a live symbol (SMART, `trading_class == symbol` preferred).
struct ChainPlan {
    input_index: usize,
    symbol: String,
    spot: f64,
    expiry: String,
    dte: i64,
    trading_class: String,
    strikes: Vec<f64>,
}

/// A chain row as drained from reqSecDefOptParams (SMART rows only).
#[derive(Clone)]
struct ChainInfo {
    trading_class: String,
    expirations: Vec<String>,
    strikes: Vec<f64>,
}

/// `omi pcs-scan SYMBOL...` — one connection, one md-type switch, then the ADR 0039 flow:
/// spot pass → chain step → pass 1 (coarse) → pass 2 (refine) → select/shape. Usage errors are
/// raised BEFORE connect (frozen); every per-symbol/per-contract failure becomes a skip row.
pub fn pcs_scan(cfg: &Config, args: &PcsScanArgs) -> Result<Value, AppError> {
    let p = PcsParams {
        dte_min: args.dte_min,
        dte_max: args.dte_max,
        dte_target: args.dte_target,
        delta: args.delta,
        width: args.width,
        min_credit_ratio: args.min_credit_ratio,
    };
    // Pre-connect validation (usage envelope, exit 64) — ordering is frozen: a bad flag must
    // never reach `connect` (tests/pcs_scan.rs usage_errors_precede_connect).
    validate_params(&p).map_err(|message| AppError::usage(message, "pcs-scan"))?;

    let client = super::connect(cfg)?;
    let market_data_type = match cfg.md_type {
        MdType::Live => MarketDataType::Realtime,
        MdType::Delayed => MarketDataType::Delayed,
        MdType::Frozen => MarketDataType::Frozen,
    };
    client
        .switch_market_data_type(market_data_type)
        .map_err(|e| AppError::data(format!("switch_market_data_type failed: {e}"), "pcs-scan"))?;

    let today = time::OffsetDateTime::now_utc().date();
    // (input index, symbol, reason) — sorted back into input order before shaping.
    let mut skipped: Vec<(usize, String, SkipReason)> = Vec::new();

    // Step 1 — spot pass: one batched stock-snapshot run for ALL symbols (ADR 0039).
    let stock_contracts: Vec<Contract> = args
        .symbols
        .iter()
        .map(|symbol| Contract::stock(symbol).build())
        .collect();
    let spot_results = snapshot_batch(&client, &stock_contracts);
    let mut live: Vec<(usize, String, f64)> = Vec::new();
    for ((input_index, symbol), result) in args.symbols.iter().enumerate().zip(spot_results) {
        match result.ok().and_then(|data| pick_spot(&data.ticks)) {
            Some(spot) => live.push((input_index, symbol.clone(), spot)),
            None => skipped.push((input_index, symbol.clone(), SkipReason::SpotUnavailable)),
        }
    }

    // Step 2 — chain step (sequential, same connection): conid → option chain → SMART row →
    // expiry pick. Every failure here is per-symbol (skip-not-fail).
    let mut plans: Vec<ChainPlan> = Vec::new();
    for (input_index, symbol, spot) in live {
        let chain = match fetch_chain_info(&client, &symbol) {
            Ok(chain) => chain,
            Err(reason) => {
                skipped.push((input_index, symbol, reason));
                continue;
            }
        };
        match pick_expiry(&chain.expirations, today, &p) {
            Some((expiry, dte)) => plans.push(ChainPlan {
                input_index,
                symbol,
                spot,
                expiry,
                dte,
                trading_class: chain.trading_class,
                strikes: chain.strikes,
            }),
            None => skipped.push((input_index, symbol, SkipReason::NoExpiryInWindow)),
        }
    }

    // Step 3 — pass 1 (coarse): up to 8 strikes per symbol, batched across all symbols.
    let mut pass1: Vec<PlannedContract> = Vec::new();
    let mut coarse_by_plan: Vec<Vec<f64>> = Vec::with_capacity(plans.len());
    for (plan_index, plan) in plans.iter().enumerate() {
        let coarse = coarse_strikes(&plan.strikes, plan.spot, 8);
        coarse_by_plan.push(coarse.clone());
        for strike in coarse {
            if let Some(contract) = option_contract(plan, strike) {
                pass1.push(PlannedContract {
                    plan_index,
                    strike,
                    contract,
                });
            }
        }
    }
    let mut rows_by_plan: Vec<Vec<PutRow>> = vec![Vec::new(); plans.len()];
    let mut ok_by_plan: Vec<usize> = vec![0; plans.len()];
    let pass1_contracts: Vec<Contract> = pass1
        .iter()
        .map(|planned| planned.contract.clone())
        .collect();
    for (planned, result) in pass1.iter().zip(snapshot_batch(&client, &pass1_contracts)) {
        if let Ok(data) = result {
            rows_by_plan[planned.plan_index].push(put_row(planned.strike, &data));
            ok_by_plan[planned.plan_index] += 1;
        }
    }

    // Step 4 — pass 2 (refine): 3 nearest strikes to the interpolated k* + their −width
    // partners, batched across all symbols. Symbols without a delta curve have nothing to
    // refine (their selection falls through to `NoGreeks`).
    let mut pass2: Vec<PlannedContract> = Vec::new();
    for (plan_index, plan) in plans.iter().enumerate() {
        let Some(k_star) = estimate_target_strike(&rows_by_plan[plan_index], p.delta) else {
            continue;
        };
        let refine = refine_strikes(
            &plan.strikes,
            k_star,
            p.width,
            3,
            &coarse_by_plan[plan_index],
        );
        for strike in refine {
            if let Some(contract) = option_contract(plan, strike) {
                pass2.push(PlannedContract {
                    plan_index,
                    strike,
                    contract,
                });
            }
        }
    }
    if !pass2.is_empty() {
        let pass2_contracts: Vec<Contract> = pass2
            .iter()
            .map(|planned| planned.contract.clone())
            .collect();
        for (planned, result) in pass2.iter().zip(snapshot_batch(&client, &pass2_contracts)) {
            // A failed pass-2 contract simply contributes no row (the strike is absent).
            if let Ok(data) = result {
                rows_by_plan[planned.plan_index].push(put_row(planned.strike, &data));
            }
        }
    }

    // Step 5 — select over pass1 ∪ pass2, with the ADR 0039 §3 skip precedence: a symbol whose
    // EVERY pass-1 contract failed is `quote_error` before the delta-availability checks.
    let mut candidates: Vec<PcsCandidate> = Vec::new();
    for (plan_index, plan) in plans.iter().enumerate() {
        if ok_by_plan[plan_index] == 0 {
            skipped.push((plan.input_index, plan.symbol.clone(), SkipReason::QuoteError));
            continue;
        }
        match select_pcs(
            &plan.symbol,
            plan.spot,
            &plan.expiry,
            plan.dte,
            &rows_by_plan[plan_index],
            &p,
        ) {
            Ok(candidate) => candidates.push(candidate),
            Err(reason) => skipped.push((plan.input_index, plan.symbol.clone(), reason)),
        }
    }

    skipped.sort_by_key(|(input_index, _, _)| *input_index);
    let skipped: Vec<(String, SkipReason)> = skipped
        .into_iter()
        .map(|(_, symbol, reason)| (symbol, reason))
        .collect();
    Ok(shape_pcs_scan(&p, candidates, skipped))
}

/// One enumerated contract: which plan it belongs to, its strike, and the resolved IB contract.
struct PlannedContract {
    plan_index: usize,
    strike: f64,
    contract: Contract,
}

/// Build the option contract for `plan` at `strike` (SMART/USD, the plan's trading class).
/// `None` only if the plan's expiry is not a valid YYYYMMDD (impossible after `pick_expiry`).
fn option_contract(plan: &ChainPlan, strike: f64) -> Option<Contract> {
    let (year, month, day) = parse_expiry(&plan.expiry)?;
    Some(
        Contract::put(&plan.symbol)
            .strike(strike)
            .expires_on(year, month, day)
            .on_exchange("SMART")
            .in_currency("USD")
            .trading_class(&plan.trading_class)
            .build(),
    )
}

/// Resolve the underlying conid, drain reqSecDefOptParams (ADR 0016 timeout posture), and pick
/// the SMART chain row: exact `trading_class == symbol` first (avoids odd weekly classes like
/// `2NVDA`), else the SMART row with the most expirations. Any failure ⇒ `quote_error`.
fn fetch_chain_info(client: &Client, symbol: &str) -> Result<ChainInfo, SkipReason> {
    let underlying = Contract::stock(symbol).build();
    let conid = client
        .contract_details(&underlying)
        .map_err(|_| SkipReason::QuoteError)?
        .first()
        .map(|details| details.contract.contract_id)
        .ok_or(SkipReason::QuoteError)?;

    let subscription = client
        .option_chain(symbol, "", SecurityType::Stock, conid)
        .map_err(|_| SkipReason::QuoteError)?;

    let mut smart_rows: Vec<ChainInfo> = Vec::new();
    let mut items = subscription.timeout_iter_data(super::TAKE_FIRST_TIMEOUT);
    loop {
        let waited = Instant::now();
        match items.next() {
            Some(Ok(chain)) => {
                if chain.exchange == "SMART" {
                    smart_rows.push(ChainInfo {
                        trading_class: chain.trading_class,
                        expirations: chain.expirations,
                        strikes: chain.strikes,
                    });
                }
            }
            Some(Err(_)) => return Err(SkipReason::QuoteError),
            None if waited.elapsed() >= super::TAKE_FIRST_TIMEOUT => {
                return Err(SkipReason::QuoteError)
            }
            None => break, // instant None = stream self-ended on the End marker => done
        }
    }

    smart_rows
        .iter()
        .find(|row| row.trading_class == symbol)
        .cloned()
        .or_else(|| {
            smart_rows
                .into_iter()
                .max_by_key(|row| (row.expirations.len(), row.trading_class.clone()))
        })
        .ok_or(SkipReason::QuoteError)
}

/// PutRow from a contract's snapshot: bid/ask ticks (Delayed*-tolerant) + model delta/IV.
fn put_row(strike: f64, data: &SnapData) -> PutRow {
    PutRow {
        strike,
        bid: tick_price(&data.ticks, "Bid"),
        ask: tick_price(&data.ticks, "Ask"),
        delta: data.greeks.as_ref().and_then(|greeks| greeks.delta),
        iv: data
            .greeks
            .as_ref()
            .and_then(|greeks| greeks.implied_volatility),
    }
}

/// Batched snapshots on the ONE blocking client (ADR 0039): chunks of [`PCS_BATCH_LINES`],
/// all subscriptions opened first (they run in parallel server-side — each owns its
/// request-id channel), then drained against ONE shared chunk deadline (ADR 0038). A
/// subscribe error, a stream error, or a missing SnapshotEnd ⇒ that contract's `Err` only.
/// The chunk is dropped (CancelMktData on unfinished snapshots) before the next batch.
fn snapshot_batch(client: &Client, contracts: &[Contract]) -> Vec<Result<SnapData, SkipReason>> {
    let mut results = Vec::with_capacity(contracts.len());
    for chunk in contracts.chunks(PCS_BATCH_LINES) {
        let mut subs: Vec<Option<Subscription<TickTypes>>> = Vec::with_capacity(chunk.len());
        for (position, contract) in chunk.iter().enumerate() {
            match client.market_data(contract).snapshot().subscribe() {
                Ok(sub) => subs.push(Some(sub)),
                Err(_) => subs.push(None),
            }
            if position + 1 < chunk.len() {
                std::thread::sleep(SUBSCRIBE_SPACING);
            }
        }
        let deadline = Instant::now() + super::SNAPSHOT_DEADLINE;
        for sub in subs {
            match sub {
                Some(sub) => results.push(drain_snapshot(&sub, deadline)),
                None => results.push(Err(SkipReason::QuoteError)),
            }
        }
        // All `subs` dropped by now: unfinished snapshots send CancelMktData before the next chunk.
    }
    results
}

/// One read from a snapshot subscription, decoupled from ibapi so the shared-deadline drain
/// schedule is unit-testable with plain data (repo no-mock rule).
enum DrainRead<T> {
    /// `TickTypes::SnapshotEnd` — the snapshot's terminal marker.
    End,
    /// A tick to record.
    Data(T),
    /// A notice: ignored, stream continues (ADR 0038 parity).
    Notice,
    /// A terminal stream error: this contract's `quote_error`.
    Error,
}

/// Map a raw subscription read to [`DrainRead`]; generic over the error type so the blocking and
/// the non-blocking read feed one schedule.
fn to_drain_read<E>(item: Result<SubscriptionItem<TickTypes>, E>) -> DrainRead<TickTypes> {
    match item {
        Ok(SubscriptionItem::Data(TickTypes::SnapshotEnd)) => DrainRead::End,
        Ok(SubscriptionItem::Data(tick)) => DrainRead::Data(tick),
        Ok(SubscriptionItem::Notice(_)) => DrainRead::Notice,
        Err(_) => DrainRead::Error,
    }
}

/// The ADR 0038 loop for one subscription against the chunk's shared `deadline`, in two phases:
/// blocking `next_timeout(remaining)` while time remains, then NON-blocking `try_next()` once the
/// deadline has passed. `next_timeout(0)` must not be used past the deadline — ibapi returns `None`
/// immediately without reading items already buffered (ibapi-3.1.0 `subscriptions/sync.rs:222-231`),
/// which skip-failed every later subscription of a chunk whose head held the shared deadline.
/// `SnapshotEnd` in either phase ⇒ `Ok`; the post-deadline buffer exhausted without `SnapshotEnd`
/// ⇒ `quote_error` (ADR 0038); pre-deadline `None` (stream self-ended) ⇒ `Ok` (ADR 0016).
/// A blocking `None` is classified AFTER the call by `Instant::now()` — never by the pre-call
/// `remaining`, which cannot tell a self-end from a read that slept out the window. Waited out ⇒
/// deadline timeout ⇒ fall through to the buffered phase; still early ⇒ self-ended `Ok`.
fn drain_until_end<T>(
    deadline: Instant,
    mut blocking: impl FnMut(Duration) -> Option<DrainRead<T>>,
    mut buffered: impl FnMut() -> Option<DrainRead<T>>,
    mut on_data: impl FnMut(T),
) -> Result<(), SkipReason> {
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            // Past the shared deadline: NON-blocking reads only, and an exhausted buffer here IS
            // this contract's timeout (ADR 0038).
            match buffered() {
                Some(DrainRead::End) => return Ok(()),
                Some(DrainRead::Data(item)) => on_data(item),
                Some(DrainRead::Notice) => {}
                Some(DrainRead::Error) | None => return Err(SkipReason::QuoteError),
            }
        } else {
            match blocking(remaining) {
                Some(DrainRead::End) => return Ok(()),
                Some(DrainRead::Data(item)) => on_data(item),
                Some(DrainRead::Notice) => {}
                Some(DrainRead::Error) => return Err(SkipReason::QuoteError),
                // Re-check the clock AFTER the call: a read that slept out `remaining` is the
                // deadline timeout and falls through to the buffered phase on the next iteration;
                // only a `None` returned while time remains is the stream self-end.
                None if Instant::now() < deadline => return Ok(()),
                None => {}
            }
        }
    }
}

/// The ADR 0038 loop for one subscription: drain until `SnapshotEnd` under the shared
/// `deadline`; `Some(Err)` and deadline expiry are this contract's `quote_error`.
fn drain_snapshot(sub: &Subscription<TickTypes>, deadline: Instant) -> Result<SnapData, SkipReason> {
    let mut ticks: Map<String, Value> = Map::new();
    let mut greeks: Option<GreeksRow> = None;
    drain_until_end(
        deadline,
        |remaining| sub.next_timeout(remaining).map(to_drain_read),
        || sub.try_next().map(to_drain_read),
        |tick| {
            if let Some((label, price)) = quote_price_tick(&tick) {
                ticks.insert(label, json!(price));
            }
            if let Some(row) = option_quote_greeks(&tick) {
                greeks = Some(row); // last-model-row-wins (ADR 0019 D3)
            }
        },
    )?;
    Ok(SnapData { ticks, greeks })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    /// Review-01 F1 regression: a sibling whose ticks + SnapshotEnd already arrived inside the
    /// shared window must be READ after the deadline. The 68ad974 loop called `next_timeout(0)`,
    /// got `None` without reading the buffer, and skip-failed the contract.
    #[test]
    fn drain_until_end_reads_buffered_items_after_deadline() {
        let deadline = Instant::now() - Duration::from_millis(1);
        let mut buffered =
            VecDeque::from([DrainRead::Data(1_u32), DrainRead::Data(2), DrainRead::End]);
        let mut seen: Vec<u32> = Vec::new();
        let result = drain_until_end::<u32>(
            deadline,
            |_: Duration| None,
            || buffered.pop_front(),
            |item| seen.push(item),
        );
        assert!(result.is_ok(), "buffered SnapshotEnd => Ok");
        assert_eq!(
            seen,
            vec![1, 2],
            "every tick buffered inside the window is read"
        );
        assert!(buffered.is_empty(), "drain stops at SnapshotEnd");
    }

    /// Past the deadline with nothing buffered: still this contract's timeout (ADR 0038).
    #[test]
    fn drain_until_end_past_deadline_empty_buffer_is_quote_error() {
        let result = drain_until_end::<u32>(
            Instant::now() - Duration::from_millis(1),
            |_: Duration| None,
            || None,
            |_| {},
        );
        assert_eq!(result, Err(SkipReason::QuoteError));
    }

    /// Pre-deadline scheduling unchanged: `SnapshotEnd` ⇒ Ok, `Some(Err)` ⇒ quote_error, stream
    /// self-end (`None`) ⇒ Ok; the non-blocking read is never consulted while time remains.
    #[test]
    fn drain_until_end_before_deadline_keeps_adr0038_semantics() {
        let deadline = Instant::now() + Duration::from_secs(60);
        let mut blocking =
            VecDeque::from([DrainRead::Data(7_u32), DrainRead::Notice, DrainRead::End]);
        let mut seen: Vec<u32> = Vec::new();
        let result = drain_until_end::<u32>(
            deadline,
            |_| blocking.pop_front(),
            || panic!("buffered read must not run before the deadline"),
            |item| seen.push(item),
        );
        assert!(result.is_ok());
        assert_eq!(seen, vec![7]);

        let result = drain_until_end::<u32>(
            deadline,
            |_| Some(DrainRead::Error),
            || panic!("buffered read must not run before the deadline"),
            |_| {},
        );
        assert_eq!(result, Err(SkipReason::QuoteError));

        let result = drain_until_end::<u32>(
            deadline,
            |_: Duration| None, // pre-deadline None = stream self-ended => success
            || panic!("buffered read must not run before the deadline"),
            |_| {},
        );
        assert!(result.is_ok());
    }

    /// Review-02 F2 regression: a blocking read that SLEEPS OUT `remaining` and returns `None`
    /// (a real ibapi timeout) must classify as this contract's timeout. At 7426cb1 `remaining`
    /// was captured before the call, so the post-call `None` still saw a non-zero `remaining`
    /// and returned `Ok(())` (an empty-but-"successful" snapshot).
    #[test]
    fn drain_until_end_blocking_none_at_deadline_is_quote_error() {
        let deadline = Instant::now() + Duration::from_millis(50);
        let result = drain_until_end::<u32>(
            deadline,
            |remaining| {
                std::thread::sleep(remaining);
                None
            },
            || None,
            |_| {},
        );
        assert_eq!(result, Err(SkipReason::QuoteError));
    }

    /// A blocking timeout (`None` at/after the deadline) falls through to the buffered phase and
    /// reads what arrived inside the window: `Data` + `SnapshotEnd` ⇒ `Ok` with that data.
    #[test]
    fn drain_until_end_blocking_timeout_falls_through_to_buffer() {
        let deadline = Instant::now() + Duration::from_millis(50);
        let mut buffered = VecDeque::from([DrainRead::Data(9_u32), DrainRead::End]);
        let mut seen: Vec<u32> = Vec::new();
        let result = drain_until_end::<u32>(
            deadline,
            |remaining| {
                std::thread::sleep(remaining);
                None
            },
            || buffered.pop_front(),
            |item| seen.push(item),
        );
        assert!(result.is_ok(), "buffered SnapshotEnd after a timeout => Ok");
        assert_eq!(seen, vec![9], "ticks buffered inside the window are recorded");
        assert!(buffered.is_empty(), "drain stops at SnapshotEnd");
    }

    #[test]
    fn tick_price_prefers_exact_label_over_delayed() {
        let ticks: Map<String, Value> = [
            ("Bid".to_string(), json!(1.5)),
            ("DelayedBid".to_string(), json!(1.4)),
            ("DelayedAsk".to_string(), json!(1.6)),
        ]
        .into_iter()
        .collect();
        assert_eq!(tick_price(&ticks, "Bid"), Some(1.5));
        assert_eq!(tick_price(&ticks, "Ask"), Some(1.6));
        assert_eq!(tick_price(&ticks, "Last"), None);
    }

    #[test]
    fn parse_yyyymmdd_rejects_malformed_and_impossible_dates() {
        assert!(parse_yyyymmdd("20261023").is_some());
        assert!(parse_yyyymmdd("bad").is_none());
        assert!(parse_yyyymmdd("20261301").is_none());
        assert!(parse_yyyymmdd("20260230").is_none());
    }

    #[test]
    fn coarse_strikes_single_slot_keeps_band_entry() {
        let strikes: Vec<f64> = (60..=100).map(f64::from).collect();
        assert_eq!(coarse_strikes(&strikes, 100.0, 1), vec![65.0]);
        assert!(coarse_strikes(&strikes, 100.0, 0).is_empty());
    }
}
