//! `delta-legs` — read-only CC/CSP leg picker inside a ~0.15–0.25 |δ| band.
//!
//! Two layers, same split as `pcs_scan`:
//! - PURE seams (frozen by `tests/delta_legs.rs`): param validation, leg selection
//!   + annualization, output shape.
//! - GATEWAY flow (`delta_legs`): one connection, one md-type switch, one stock
//!   snapshot, one SMART chain, one batched option snapshot, then the pure seams.
//!
//! Read-only: this module places no orders. Ranking is a sort key, NOT a trade
//! recommendation — the disclaimer says so in both languages.
//!
//! Constants:
//! - Default |δ| band is **inclusive** `[DELTA_MIN, DELTA_MAX]` = `[0.15, 0.25]`.
//! - Annualization target midpoint is `SORT_AIM` = 0.175 (the middle of the 15–20%
//!   operator target); the hard floor is `FLOOR` = 0.10.
//! - Annualized yield, one contract: call `(mid / spot) × (365 / dte)`; put
//!   `(mid / strike) × (365 / dte)` (premium against assignment notional per share).
//!   `mid` is `(bid + ask) / 2` when both are `> 0`, else the leg is not usable.
//! - |δ| is `delta.abs()`. A leg is in band iff `delta_min <= |δ| <= delta_max`.

use std::cmp::Ordering;
use std::time::{Duration, Instant};

use ibapi::client::blocking::Client;
use ibapi::contracts::SecurityType;
use ibapi::market_data::MarketDataType;
use ibapi::prelude::{Contract, Subscription, SubscriptionItem, TickTypes};
use serde_json::{json, Map, Value};

use crate::cli::{DeltaLegsArgs, DeltaLegsKind};
use crate::config::{Config, MdType};
use crate::error::AppError;

use super::assignment_notional;
use super::option_quote::{option_quote_greeks, parse_expiry};
use super::pcs_scan::{pick_expiry, pick_spot, PcsParams};
use super::quote_price_tick;

/// Inclusive lower edge of the default |δ| band.
pub const DELTA_MIN: f64 = 0.15;
/// Inclusive upper edge of the default |δ| band.
pub const DELTA_MAX: f64 = 0.25;
/// Hard floor on annualized yield. Below this a leg is flagged, never dropped.
pub const FLOOR: f64 = 0.10;
/// Lower edge of the operator's annualized target band.
pub const TARGET_MIN: f64 = 0.15;
/// Upper edge of the operator's annualized target band.
pub const TARGET_MAX: f64 = 0.20;
/// Sort aim: the midpoint of the 15–20% target. Closer ranks first.
pub const SORT_AIM: f64 = 0.175;
/// |δ| the secondary sort pulls toward.
const DELTA_AIM: f64 = 0.20;

/// Max option contracts quoted per side. Below IB's default 100 market-data lines.
const QUOTE_CAP: usize = 30;
/// Inter-subscribe spacing (IB paces market-data requests ~50 msgs/s).
const SUBSCRIBE_SPACING: Duration = Duration::from_millis(20);

/// Which side to rank.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegKind {
    /// Covered calls only.
    Covered,
    /// Cash-secured puts only.
    Csp,
    /// Both sides.
    Both,
}

impl LegKind {
    fn wants_call(self) -> bool {
        matches!(self, LegKind::Covered | LegKind::Both)
    }
    fn wants_put(self) -> bool {
        matches!(self, LegKind::Csp | LegKind::Both)
    }
}

/// Validated selection parameters. Defaults: DTE 21/45/30, |δ| band 0.15–0.25, both sides.
#[derive(Debug, Clone, PartialEq)]
pub struct DeltaLegsParams {
    pub dte_min: i64,
    pub dte_max: i64,
    pub dte_target: i64,
    pub delta_min: f64,
    pub delta_max: f64,
    pub kind: LegKind,
}

impl Default for DeltaLegsParams {
    fn default() -> Self {
        Self {
            dte_min: 21,
            dte_max: 45,
            dte_target: 30,
            delta_min: DELTA_MIN,
            delta_max: DELTA_MAX,
            kind: LegKind::Both,
        }
    }
}

/// One fetched option leg. `bid`/`ask`/`delta`/`iv` are `None` when the snapshot
/// never carried that datum. `right` is `"C"` or `"P"`.
#[derive(Debug, Clone, PartialEq)]
pub struct LegRow {
    pub strike: f64,
    pub bid: Option<f64>,
    pub ask: Option<f64>,
    pub delta: Option<f64>,
    pub iv: Option<f64>,
    pub right: String,
}

/// One leg inside the |δ| band, ranked. `band` is always `"recommended"` — a leg
/// outside the band is never constructed.
#[derive(Debug, Clone, PartialEq)]
pub struct LegCandidate {
    pub symbol: String,
    pub spot: f64,
    pub expiry: String,
    pub dte: i64,
    pub right: String,
    pub strike: f64,
    pub bid: f64,
    pub ask: f64,
    pub mid: f64,
    pub delta: f64,
    pub iv: Option<f64>,
    pub abs_delta: f64,
    pub annualized: Option<f64>,
    /// `strike × 100 × 1` for puts; `None` for calls (no assignment).
    pub assignment_notional: Option<f64>,
    pub in_target_band: bool,
    pub above_floor: bool,
    pub band: &'static str,
}

/// Pre-connect validation (usage error, exit 64). DTE rules match `pcs-scan`;
/// both delta edges must be finite, in `(0, 1)`, and `min <= max`.
pub fn validate_delta_legs_params(p: &DeltaLegsParams) -> Result<(), String> {
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
    for (flag, value) in [("--delta-min", p.delta_min), ("--delta-max", p.delta_max)] {
        if !value.is_finite() || value <= 0.0 || value >= 1.0 {
            return Err(format!("{flag} must be in (0,1) (got {value})"));
        }
    }
    if p.delta_min > p.delta_max {
        return Err(format!(
            "--delta-min ({}) must be <= --delta-max ({})",
            p.delta_min, p.delta_max
        ));
    }
    Ok(())
}

/// `(bid + ask) / 2` when both are present and `> 0`; else the leg cannot be ranked.
fn usable_mid(row: &LegRow) -> Option<f64> {
    match (row.bid, row.ask) {
        (Some(bid), Some(ask)) if bid > 0.0 && ask > 0.0 => Some((bid + ask) / 2.0),
        _ => None,
    }
}

/// Annualized premium yield for one contract, or `None` when an input is unusable.
/// Call: premium over spot. Put: premium over the per-share assignment notional.
fn annualized(right: &str, mid: f64, spot: f64, strike: f64, dte: i64) -> Option<f64> {
    if dte <= 0 || !mid.is_finite() {
        return None;
    }
    let base = match right {
        "C" if spot > 0.0 => spot,
        "P" if strike > 0.0 => strike,
        _ => return None,
    };
    Some((mid / base) * (365.0 / dte as f64))
}

/// Legs inside the inclusive |δ| band, filtered by side, ranked toward the 17.5% aim.
///
/// A row is kept only when its mid is usable, its delta is present, and
/// `delta_min <= |δ| <= delta_max`. Out-of-band rows are dropped, never returned.
/// Sort: distance of annualized yield to [`SORT_AIM`] ascending (a missing yield
/// sorts last), then `| |δ| − 0.20 |` ascending, then strike ascending.
pub fn select_delta_legs(
    symbol: &str,
    spot: f64,
    expiry: &str,
    dte: i64,
    rows: &[LegRow],
    p: &DeltaLegsParams,
) -> Vec<LegCandidate> {
    let mut out: Vec<LegCandidate> = rows
        .iter()
        .filter(|row| match row.right.as_str() {
            "C" => p.kind.wants_call(),
            "P" => p.kind.wants_put(),
            _ => false,
        })
        .filter_map(|row| {
            let mid = usable_mid(row)?;
            let delta = row.delta.filter(|d| d.is_finite())?;
            let abs_delta = delta.abs();
            if abs_delta < p.delta_min || abs_delta > p.delta_max {
                return None;
            }
            let (bid, ask) = (row.bid?, row.ask?);
            let yield_ = annualized(&row.right, mid, spot, row.strike, dte);
            Some(LegCandidate {
                symbol: symbol.to_string(),
                spot,
                expiry: expiry.to_string(),
                dte,
                right: row.right.clone(),
                strike: row.strike,
                bid,
                ask,
                mid,
                delta,
                iv: row.iv.filter(|v| v.is_finite()),
                abs_delta,
                annualized: yield_,
                assignment_notional: (row.right == "P")
                    .then(|| assignment_notional(row.strike, 1.0)),
                in_target_band: yield_.is_some_and(|y| (TARGET_MIN..=TARGET_MAX).contains(&y)),
                above_floor: yield_.is_some_and(|y| y >= FLOOR),
                band: "recommended",
            })
        })
        .collect();

    out.sort_by(|a, b| {
        // Scale to integers: f64 distances that are equal on paper (0.3 − 0.175 and
        // 0.175 − 0.05 both print 0.125) do not compare equal in binary.
        cmp_scaled(aim_distance(a), aim_distance(b)).then_with(|| {
            cmp_scaled((a.abs_delta - DELTA_AIM).abs(), (b.abs_delta - DELTA_AIM).abs())
                .then_with(|| cmp_scaled(a.strike, b.strike))
        })
    });
    out
}

/// Ascending compare of two finite floats at 1e-9 resolution.
fn cmp_scaled(a: f64, b: f64) -> Ordering {
    ((a * 1e9).round() as i64).cmp(&((b * 1e9).round() as i64))
}

/// Distance to the 17.5% aim; `f64::MAX` puts a leg with no yield last.
fn aim_distance(c: &LegCandidate) -> f64 {
    c.annualized.map(|y| (y - SORT_AIM).abs()).unwrap_or(f64::MAX)
}

fn candidate_json(c: &LegCandidate) -> Value {
    json!({
        "symbol": c.symbol,
        "spot": c.spot,
        "expiry": c.expiry,
        "dte": c.dte,
        "right": c.right,
        "strike": c.strike,
        "bid": c.bid,
        "ask": c.ask,
        "mid": c.mid,
        "delta": c.delta,
        "iv": c.iv,
        "abs_delta": c.abs_delta,
        "annualized": c.annualized,
        "assignment_notional": c.assignment_notional,
        "in_target_band": c.in_target_band,
        "above_floor": c.above_floor,
        "band": c.band,
    })
}

/// Final output shape. `recommended` aliases `candidates` (the 推荐带);
/// `out_of_band` is always empty — legs outside the |δ| band are omitted, not listed.
pub fn shape_delta_legs(
    p: &DeltaLegsParams,
    symbol: &str,
    spot: Option<f64>,
    expiry: Option<&str>,
    dte: Option<i64>,
    candidates: &[LegCandidate],
    skipped_note: Option<&str>,
) -> Value {
    let candidates_json: Vec<Value> = candidates.iter().map(candidate_json).collect();
    let mut out = json!({
        "symbol": symbol,
        "spot": spot,
        "expiry": expiry,
        "dte": dte,
        "params": {
            "dte_min": p.dte_min,
            "dte_max": p.dte_max,
            "dte_target": p.dte_target,
            "delta_min": p.delta_min,
            "delta_max": p.delta_max,
            "delta_band_inclusive": true,
            "kind": match p.kind {
                LegKind::Covered => "covered",
                LegKind::Csp => "csp",
                LegKind::Both => "both",
            },
        },
        "delta_band": { "min": p.delta_min, "max": p.delta_max, "inclusive": true },
        "kpi": {
            "floor": FLOOR,
            "target_min": TARGET_MIN,
            "target_max": TARGET_MAX,
            "sort_aim": SORT_AIM,
        },
        "greeks_source": "IB ModelOption(13) / DelayedModelOption(83) via option snapshot, last-write-wins; a leg with no model delta is omitted from the recommended band (no synthetic Black-Scholes)",
        "disclaimer": "read-only ranking; sorting is not a trade recommendation / 排序不是建议下单. To trade, use covered-call/short-put preview (--confirm to send).",
        "candidates": candidates_json.clone(),
        "recommended": candidates_json,
        "out_of_band": [],
    });
    if let Some(note) = skipped_note {
        out["degraded"] = json!(note);
    }
    out
}

// ---------------------------------------------------------------------------
// Gateway flow (NOT frozen — review-by-reading + operator paper acceptance)
// ---------------------------------------------------------------------------

struct SnapData {
    ticks: Map<String, Value>,
    delta: Option<f64>,
    iv: Option<f64>,
}

#[derive(Clone)]
struct ChainInfo {
    trading_class: String,
    expirations: Vec<String>,
    strikes: Vec<f64>,
}

struct PlannedContract {
    strike: f64,
    right: &'static str,
    contract: Contract,
}

/// `omi delta-legs SYMBOL` — validate, then one connection: md-type switch, stock
/// snapshot, SMART chain, expiry pick, one batched option snapshot over a thinned
/// OTM window, then select + shape. No model greeks ⇒ empty candidates and
/// `degraded: "no_model_greeks"`, still exit 0.
pub fn delta_legs(cfg: &Config, args: &DeltaLegsArgs) -> Result<Value, AppError> {
    let p = DeltaLegsParams {
        dte_min: args.dte_min,
        dte_max: args.dte_max,
        dte_target: args.dte_target,
        delta_min: args.delta_min,
        delta_max: args.delta_max,
        kind: match args.kind {
            DeltaLegsKind::Covered => LegKind::Covered,
            DeltaLegsKind::Csp => LegKind::Csp,
            DeltaLegsKind::Both => LegKind::Both,
        },
    };
    // Pre-connect (usage envelope, exit 64): a bad flag must never reach `connect`.
    validate_delta_legs_params(&p).map_err(|message| AppError::usage(message, "delta-legs"))?;

    let client = super::connect(cfg)?;
    let market_data_type = match cfg.md_type {
        MdType::Live => MarketDataType::Realtime,
        MdType::Delayed => MarketDataType::Delayed,
        MdType::Frozen => MarketDataType::Frozen,
    };
    client
        .switch_market_data_type(market_data_type)
        .map_err(|e| {
            AppError::data(format!("switch_market_data_type failed: {e}"), "delta-legs")
        })?;

    let symbol = args.symbol.to_ascii_uppercase();
    let stock = Contract::stock(&symbol).build();
    let spot = match snapshot_batch(&client, &[stock])
        .into_iter()
        .next()
        .and_then(Result::ok)
        .and_then(|data| pick_spot(&data.ticks))
    {
        Some(spot) => spot,
        None => {
            return Ok(shape_delta_legs(
                &p,
                &symbol,
                None,
                None,
                None,
                &[],
                Some("spot_unavailable"),
            ))
        }
    };

    let chain = match fetch_chain_info(&client, &symbol) {
        Ok(chain) => chain,
        Err(()) => {
            return Ok(shape_delta_legs(
                &p,
                &symbol,
                Some(spot),
                None,
                None,
                &[],
                Some("chain_unavailable"),
            ))
        }
    };

    let today = time::OffsetDateTime::now_utc().date();
    let dte_params = PcsParams {
        dte_min: p.dte_min,
        dte_max: p.dte_max,
        dte_target: p.dte_target,
        ..PcsParams::default()
    };
    let Some((expiry, dte)) = pick_expiry(&chain.expirations, today, &dte_params) else {
        return Ok(shape_delta_legs(
            &p,
            &symbol,
            Some(spot),
            None,
            None,
            &[],
            Some("no_expiry_in_window"),
        ));
    };

    let planned = plan_contracts(&symbol, spot, &expiry, &chain, p.kind);
    let contracts: Vec<Contract> = planned.iter().map(|c| c.contract.clone()).collect();
    let mut rows: Vec<LegRow> = Vec::new();
    let mut saw_delta = false;
    for (planned, result) in planned.iter().zip(snapshot_batch(&client, &contracts)) {
        let Ok(data) = result else { continue };
        if data.delta.is_some() {
            saw_delta = true;
        }
        rows.push(LegRow {
            strike: planned.strike,
            bid: tick_price(&data.ticks, "Bid"),
            ask: tick_price(&data.ticks, "Ask"),
            delta: data.delta,
            iv: data.iv,
            right: planned.right.to_string(),
        });
    }

    let candidates = select_delta_legs(&symbol, spot, &expiry, dte, &rows, &p);
    // No model delta anywhere ⇒ the band is empty because greeks are missing, not
    // because nothing qualified. Say so instead of looking like a quiet zero.
    let note = if !saw_delta {
        Some("no_model_greeks")
    } else {
        None
    };
    Ok(shape_delta_legs(
        &p,
        &symbol,
        Some(spot),
        Some(&expiry),
        Some(dte),
        &candidates,
        note,
    ))
}

/// OTM window, thinned to [`QUOTE_CAP`] strikes per side: calls in `[spot, 1.20×spot]`,
/// puts in `[0.80×spot, spot]`. Both ends of the window are kept.
fn plan_contracts(
    symbol: &str,
    spot: f64,
    expiry: &str,
    chain: &ChainInfo,
    kind: LegKind,
) -> Vec<PlannedContract> {
    let mut planned = Vec::new();
    if kind.wants_call() {
        push_side(
            &mut planned,
            symbol,
            expiry,
            &chain.trading_class,
            "C",
            &window(&chain.strikes, spot, spot * 1.20),
        );
    }
    if kind.wants_put() {
        push_side(
            &mut planned,
            symbol,
            expiry,
            &chain.trading_class,
            "P",
            &window(&chain.strikes, spot * 0.80, spot),
        );
    }
    planned
}

fn push_side(
    planned: &mut Vec<PlannedContract>,
    symbol: &str,
    expiry: &str,
    trading_class: &str,
    right: &'static str,
    strikes: &[f64],
) {
    let Some((year, month, day)) = parse_expiry(expiry) else {
        return;
    };
    for &strike in strikes {
        let contract = match right {
            "C" => Contract::call(symbol),
            _ => Contract::put(symbol),
        }
        .strike(strike)
        .expires_on(year, month, day)
        .on_exchange("SMART")
        .in_currency("USD")
        .trading_class(trading_class)
        .build();
        planned.push(PlannedContract {
            strike,
            right,
            contract,
        });
    }
}

/// Strikes inside `[lower, upper]`, ascending + deduped, thinned to ≤ [`QUOTE_CAP`]
/// evenly by index (both ends kept).
fn window(strikes: &[f64], lower: f64, upper: f64) -> Vec<f64> {
    let mut band: Vec<f64> = strikes
        .iter()
        .copied()
        .filter(|s| *s >= lower && *s <= upper)
        .collect();
    band.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    band.dedup();
    thin(&band, QUOTE_CAP)
}

fn thin(band: &[f64], n: usize) -> Vec<f64> {
    if n == 0 || band.is_empty() || band.len() <= n {
        return band.to_vec();
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

fn tick_price(ticks: &Map<String, Value>, label: &str) -> Option<f64> {
    ticks
        .get(label)
        .and_then(Value::as_f64)
        .or_else(|| ticks.get(&format!("Delayed{label}")).and_then(Value::as_f64))
}

/// SMART chain row: exact `trading_class == symbol` first, else the SMART row with
/// the most expirations (same choice as `pcs_scan::fetch_chain_info`).
fn fetch_chain_info(client: &Client, symbol: &str) -> Result<ChainInfo, ()> {
    let underlying = Contract::stock(symbol).build();
    let conid = client
        .contract_details(&underlying)
        .map_err(|_| ())?
        .first()
        .map(|details| details.contract.contract_id)
        .ok_or(())?;

    let subscription = client
        .option_chain(symbol, "", SecurityType::Stock, conid)
        .map_err(|_| ())?;

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
            Some(Err(_)) => return Err(()),
            None if waited.elapsed() >= super::TAKE_FIRST_TIMEOUT => return Err(()),
            None => break,
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
        .ok_or(())
}

/// Batched snapshots, one shared deadline per chunk (the `pcs_scan` posture, kept
/// private rather than refactoring that module). A subscribe or drain failure is
/// that contract's `Err` only.
fn snapshot_batch(client: &Client, contracts: &[Contract]) -> Vec<Result<SnapData, ()>> {
    let mut results = Vec::with_capacity(contracts.len());
    for chunk in contracts.chunks(QUOTE_CAP) {
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
                None => results.push(Err(())),
            }
        }
    }
    results
}

fn drain_snapshot(sub: &Subscription<TickTypes>, deadline: Instant) -> Result<SnapData, ()> {
    let mut ticks: Map<String, Value> = Map::new();
    let mut delta: Option<f64> = None;
    let mut iv: Option<f64> = None;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(());
        }
        match sub.next_timeout(remaining) {
            Some(Ok(SubscriptionItem::Data(TickTypes::SnapshotEnd))) => break,
            Some(Ok(SubscriptionItem::Data(tick))) => {
                if let Some((label, price)) = quote_price_tick(&tick) {
                    ticks.insert(label, json!(price));
                }
                // Model rows only (ADR 0019): ModelOption / DelayedModelOption,
                // last-write-wins. No model row ⇒ delta stays None ⇒ leg omitted.
                if let Some(greeks) = option_quote_greeks(&tick) {
                    delta = greeks.delta;
                    iv = greeks.implied_volatility;
                }
            }
            Some(Ok(SubscriptionItem::Notice(_))) => {}
            Some(Err(_)) => return Err(()),
            None if Instant::now() < deadline => break,
            None => return Err(()),
        }
    }
    Ok(SnapData { ticks, delta, iv })
}
