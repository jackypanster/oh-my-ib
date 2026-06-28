//! `account` — account summary with stable agent-facing fields. (card 02)
//!
//! Uses `account_updates(account)` (reqAccountUpdates) so the result is scoped to a
//! single account by construction (honors `--account` / first managed account) and
//! carries the account base currency.

use ibapi::accounts::AccountUpdate;
use serde_json::{json, Value};

use crate::config::Config;
use crate::error::AppError;

pub fn account(cfg: &Config) -> Result<Value, AppError> {
    let client = super::connect(cfg)?;
    let account = super::resolve_account(&client, cfg)?;
    let subscription = client
        .account_updates(&account)
        .map_err(|e| AppError::data(format!("account_updates failed: {e}"), "account"))?;

    let mut net_liquidation = None;
    let mut total_cash = None;
    let mut buying_power = None;
    let mut available_funds = None;
    let mut currency = None;

    for update in subscription.iter_data() {
        let update = update
            .map_err(|e| AppError::data(format!("account_updates stream: {e}"), "account"))?;
        match update {
            AccountUpdate::AccountValue(v) => match v.key.as_str() {
                "NetLiquidation" => {
                    net_liquidation = Some(v.value);
                    if currency.is_none() && !v.currency.is_empty() {
                        currency = Some(v.currency);
                    }
                }
                "TotalCashValue" => total_cash = Some(v.value),
                "BuyingPower" => buying_power = Some(v.value),
                "AvailableFunds" => available_funds = Some(v.value),
                _ => {}
            },
            AccountUpdate::End => break,
            _ => {}
        }
    }

    Ok(json!({
        "account": account.0,
        "net_liquidation": num(net_liquidation),
        "total_cash": num(total_cash),
        "buying_power": num(buying_power),
        "available_funds": num(available_funds),
        "currency": currency,
    }))
}

/// Parse an IB string value into a JSON number, falling back to the raw string,
/// or null when the tag was absent.
fn num(value: Option<String>) -> Value {
    match value {
        Some(v) => v
            .parse::<f64>()
            .map(Value::from)
            .unwrap_or(Value::String(v)),
        None => Value::Null,
    }
}
