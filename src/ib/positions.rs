//! `positions` — current positions with valuation. (card 02)
//!
//! Uses `account_updates(account)` rather than `positions()`: the portfolio stream
//! carries market value + unrealized PnL (which `positions()` does not) and is
//! account-scoped, satisfying the documented `qty/avg_cost/market_value/unrealized_pnl`
//! shape honestly (review-01 BLOCKER 3).

use ibapi::accounts::AccountUpdate;
use serde_json::{json, Value};

use crate::config::Config;
use crate::error::AppError;

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
            AccountUpdate::PortfolioValue(p) => {
                out.push(json!({
                    "symbol": p.contract.symbol,
                    "conid": p.contract.contract_id,
                    "qty": p.position,
                    "avg_cost": p.average_cost,
                    "market_price": p.market_price,
                    "market_value": p.market_value,
                    "unrealized_pnl": p.unrealized_pnl,
                    "realized_pnl": p.realized_pnl,
                    "account": p.account,
                }));
            }
            AccountUpdate::End => break,
            _ => {}
        }
    }

    Ok(json!({ "account": account.0, "positions": out }))
}
