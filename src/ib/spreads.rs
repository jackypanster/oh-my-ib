//! `spreads` — read-only income-book view of covered calls and short puts.
//!
//! Two layers, same split as `pcs_scan`:
//! - PURE seam (`shape_spreads`): classifies drained portfolio rows. Frozen-testable
//!   without a gateway.
//! - GATEWAY flow (`spreads`): one connection, account resolution, one
//!   `account_updates` drain (the `positions` posture), then the pure seam.
//!
//! Read-only: this module places no orders and never touches `trade.rs`'s write path.
//! The only reuse is the pure `assignment_notional` (strike × 100 × qty).

use ibapi::accounts::{AccountPortfolioValue, AccountUpdate};
use ibapi::contracts::{OptionRight, SecurityType};
use serde_json::{json, Value};

use crate::config::Config;
use crate::error::AppError;

use super::assignment_notional;

/// One portfolio row reduced to what classification needs. Option fields are
/// `None` for non-OPT rows. ibapi-free so the pure seam is testable offline.
#[derive(Debug, Clone, PartialEq)]
pub struct SpreadsRow {
    pub symbol: String,
    pub sec_type: String,
    pub qty: f64,
    pub expiry: Option<String>,
    pub strike: Option<f64>,
    pub right: Option<String>,
    pub conid: i32,
    pub market_price: f64,
    pub market_value: f64,
    pub avg_cost: f64,
    pub unrealized_pnl: f64,
}

/// Map one `account_updates` portfolio value into a [`SpreadsRow`]. Right is
/// `"C"`/`"P"` for options, else `None`; non-OPT rows leave the option fields
/// empty (same identity rule as `position_row`).
pub fn spreads_row(p: &AccountPortfolioValue) -> SpreadsRow {
    let is_opt = matches!(p.contract.security_type, SecurityType::Option);
    let right = if is_opt {
        match &p.contract.right {
            Some(OptionRight::Call) => Some("C".to_string()),
            Some(OptionRight::Put) => Some("P".to_string()),
            _ => None,
        }
    } else {
        None
    };
    SpreadsRow {
        symbol: p.contract.symbol.to_string(),
        sec_type: p.contract.security_type.to_string(),
        qty: p.position,
        expiry: is_opt.then(|| p.contract.last_trade_date_or_contract_month.clone()),
        strike: is_opt.then_some(p.contract.strike),
        right,
        conid: p.contract.contract_id,
        market_price: p.market_price,
        market_value: p.market_value,
        avg_cost: p.average_cost,
        unrealized_pnl: p.unrealized_pnl,
    }
}

fn same_symbol(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}

/// Long stock held on `symbol` (short stock and non-STK rows do not cover).
fn shares_held(symbol: &str, rows: &[SpreadsRow]) -> f64 {
    rows.iter()
        .filter(|r| same_symbol(&r.symbol, symbol) && r.sec_type == "STK" && r.qty > 0.0)
        .map(|r| r.qty)
        .sum()
}

/// Contracts of short calls on `symbol`, every expiry and strike (a covered call
/// commits 100 shares per contract — same rule as `check_coverage`).
fn short_calls(symbol: &str, rows: &[SpreadsRow]) -> f64 {
    rows.iter()
        .filter(|r| {
            same_symbol(&r.symbol, symbol)
                && r.sec_type == "OPT"
                && r.right.as_deref() == Some("C")
                && r.qty < 0.0
        })
        .map(|r| r.qty.abs())
        .sum()
}

fn opt(v: Option<&str>) -> Value {
    v.map_or(Value::Null, |s| json!(s))
}

fn num_opt(v: Option<f64>) -> Value {
    v.map_or(Value::Null, |n| json!(n))
}

/// Classify the income book. `covered` is every short call with the underlying's
/// share coverage; `short_puts` is every short put with its `assignment_notional`.
/// Long options and bare stock never appear as rows — stock only feeds the
/// coverage numbers on covered rows. Both lists sort by (symbol, expiry, strike).
pub fn shape_spreads(account: &str, rows: &[SpreadsRow]) -> Value {
    let mut covered: Vec<Value> = rows
        .iter()
        .filter(|r| r.sec_type == "OPT" && r.right.as_deref() == Some("C") && r.qty < 0.0)
        .map(|r| {
            let shares = shares_held(&r.symbol, rows);
            let calls = short_calls(&r.symbol, rows);
            json!({
                "symbol": r.symbol,
                "expiry": opt(r.expiry.as_deref()),
                "strike": num_opt(r.strike),
                "qty": r.qty,
                "conid": r.conid,
                "market_price": r.market_price,
                "market_value": r.market_value,
                "avg_cost": r.avg_cost,
                "unrealized_pnl": r.unrealized_pnl,
                "shares_held": shares,
                "short_calls": calls,
                "free_shares": shares - 100.0 * calls,
                "contracts": r.qty.abs(),
            })
        })
        .collect();
    covered.sort_by_key(row_key);

    let mut short_puts: Vec<Value> = rows
        .iter()
        .filter(|r| r.sec_type == "OPT" && r.right.as_deref() == Some("P") && r.qty < 0.0)
        .map(|r| {
            let notional = r
                .strike
                .map(|strike| assignment_notional(strike, r.qty.abs()))
                .unwrap_or(0.0);
            json!({
                "symbol": r.symbol,
                "expiry": opt(r.expiry.as_deref()),
                "strike": num_opt(r.strike),
                "qty": r.qty,
                "conid": r.conid,
                "market_price": r.market_price,
                "market_value": r.market_value,
                "avg_cost": r.avg_cost,
                "unrealized_pnl": r.unrealized_pnl,
                "assignment_notional": notional,
                "contracts": r.qty.abs(),
            })
        })
        .collect();
    short_puts.sort_by_key(row_key);

    let assignment_total: f64 = short_puts
        .iter()
        .map(|row| row["assignment_notional"].as_f64().unwrap_or(0.0))
        .sum();

    json!({
        "account": account,
        "covered": covered,
        "short_puts": short_puts,
        "assignment_notional": assignment_total,
        "close_hints": {
            "covered_call": "omi covered-call close --symbol SYM --expiry YYYYMMDD --strike K --qty N --limit P",
            "short_put": "omi short-put close --symbol SYM --expiry YYYYMMDD --strike K --qty N --limit P",
            "option_close": "omi option-close --conid CONID --limit P",
        },
        "disclaimer": "read-only view; use existing covered-call/short-put/option-close preview (--confirm to send)",
    })
}

fn row_key(v: &Value) -> (String, String, i64) {
    (
        v["symbol"].as_str().unwrap_or("").to_string(),
        v["expiry"].as_str().unwrap_or("").to_string(),
        // Strikes are exchange-listed (cent precision); scale to compare without f64's Ord.
        (v["strike"].as_f64().unwrap_or(0.0) * 1000.0).round() as i64,
    )
}

/// `omi spreads` — one connection, resolve the account, drain `account_updates`
/// to `End` (same posture as `positions`), then classify. Context `"spreads"`.
pub fn spreads(cfg: &Config) -> Result<Value, AppError> {
    let client = super::connect(cfg)?;
    let account = super::resolve_account(&client, cfg)?;
    let subscription = client
        .account_updates(&account)
        .map_err(|e| AppError::data(format!("account_updates failed: {e}"), "spreads"))?;

    let mut rows = Vec::new();
    for update in subscription.iter_data() {
        let update =
            update.map_err(|e| AppError::data(format!("account_updates stream: {e}"), "spreads"))?;
        match update {
            AccountUpdate::PortfolioValue(p) => rows.push(spreads_row(&p)),
            AccountUpdate::End => break,
            _ => {}
        }
    }

    Ok(shape_spreads(&account.0, &rows))
}
