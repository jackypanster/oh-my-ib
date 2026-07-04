//! `positions` — current positions with valuation. (card 02)
//!
//! Uses `account_updates(account)` rather than `positions()`: the portfolio stream
//! carries market value + unrealized PnL (which `positions()` does not) and is
//! account-scoped, satisfying the documented `qty/avg_cost/market_value/unrealized_pnl`
//! shape honestly (review-01 BLOCKER 3). The row shape lives in `position_row` so
//! `brief`'s consolidated drain (ADR 0011) emits the SAME rows — the positions shape
//! cannot drift between `positions` and `brief`.

use ibapi::accounts::{AccountPortfolioValue, AccountUpdate};
use ibapi::contracts::{OptionRight, SecurityType};
use serde_json::{json, Value};

use crate::config::Config;
use crate::error::AppError;

/// The exact 14-key positions row (incl. per-row `account`), shared by `positions` (own
/// drain) and `brief` (consolidated drain) so the shape never drifts between them. The 5
/// identity keys: `sec_type` (ALWAYS the IB wire code via `SecurityType` Display — "STK"/
/// "OPT"/…) and `expiry`/`strike`/`right`/`multiplier`, populated IFF the contract is an
/// option, else ALL null (ADR 0022 §4). Brief parity is automatic (same fn).
pub fn position_row(p: &AccountPortfolioValue) -> Value {
    // Option identity: populated iff SecurityType::Option; non-OPT rows emit all-null.
    // right maps Call⇒"C"/Put⇒"P"; the `_` arm covers None + any non_exhaustive future
    // variant (unconstructible from tests/). multiplier is a String — empty ⇒ null
    // (house style, option_quote.rs precedent).
    let (expiry, strike, right, multiplier) =
        if matches!(p.contract.security_type, SecurityType::Option) {
            let right = match &p.contract.right {
                Some(OptionRight::Call) => json!("C"),
                Some(OptionRight::Put) => json!("P"),
                _ => Value::Null,
            };
            let multiplier = if p.contract.multiplier.is_empty() {
                Value::Null
            } else {
                json!(p.contract.multiplier)
            };
            (
                json!(p.contract.last_trade_date_or_contract_month),
                json!(p.contract.strike),
                right,
                multiplier,
            )
        } else {
            (Value::Null, Value::Null, Value::Null, Value::Null)
        };
    json!({
        "symbol": p.contract.symbol,
        "conid": p.contract.contract_id,
        "qty": p.position,
        "avg_cost": p.average_cost,
        "market_price": p.market_price,
        "market_value": p.market_value,
        "unrealized_pnl": p.unrealized_pnl,
        "realized_pnl": p.realized_pnl,
        "account": p.account,
        "sec_type": p.contract.security_type.to_string(),
        "expiry": expiry,
        "strike": strike,
        "right": right,
        "multiplier": multiplier,
    })
}

pub fn positions(cfg: &Config) -> Result<Value, AppError> {
    let client = super::connect(cfg)?;
    let account = super::resolve_account(&client, cfg)?;
    let subscription = client
        .account_updates(&account)
        .map_err(|e| AppError::data(format!("account_updates failed: {e}"), "positions"))?;

    let mut out = Vec::new();
    for update in subscription.iter_data() {
        let update = update
            .map_err(|e| AppError::data(format!("account_updates stream: {e}"), "positions"))?;
        match update {
            AccountUpdate::PortfolioValue(p) => out.push(position_row(&p)),
            AccountUpdate::End => break,
            _ => {}
        }
    }

    Ok(json!({ "account": account.0, "positions": out }))
}
