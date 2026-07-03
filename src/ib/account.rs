//! `account` — account summary with stable agent-facing fields. (card 02)
//!
//! Uses `account_updates(account)` (reqAccountUpdates) so the result is scoped to a
//! single account by construction (honors `--account` / first managed account) and
//! carries the account base currency. The AccountValue key-matching lives in
//! `SummaryAccumulator` so `brief`'s consolidated drain (ADR 0011) absorbs the SAME
//! logic — the summary shape cannot drift between `account` and `brief`.

use ibapi::accounts::AccountUpdate;
use serde_json::{json, Value};

use crate::config::Config;
use crate::error::AppError;

/// Accumulates the summary fields from AccountValue updates during an
/// `account_updates` drain. Shared by `account` (own drain) and `brief`
/// (consolidated drain, ADR 0011).
#[derive(Default)]
pub(crate) struct SummaryAccumulator {
    net_liquidation: Option<String>,
    total_cash: Option<String>,
    buying_power: Option<String>,
    available_funds: Option<String>,
    currency: Option<String>,
}

impl SummaryAccumulator {
    /// Absorb one AccountValue update. Currency is taken first-seen alongside
    /// `NetLiquidation` only (the original `account` behavior — preserved exactly).
    pub(crate) fn absorb(&mut self, key: &str, value: String, currency: String) {
        match key {
            "NetLiquidation" => {
                self.net_liquidation = Some(value);
                if self.currency.is_none() && !currency.is_empty() {
                    self.currency = Some(currency);
                }
            }
            "TotalCashValue" => self.total_cash = Some(value),
            "BuyingPower" => self.buying_power = Some(value),
            "AvailableFunds" => self.available_funds = Some(value),
            _ => {}
        }
    }

    /// The 5-key summary payload (no `account` wrapper — the hoisting rule lives
    /// in the callers).
    pub(crate) fn into_summary(self) -> Value {
        json!({
            "net_liquidation": num(self.net_liquidation),
            "total_cash": num(self.total_cash),
            "buying_power": num(self.buying_power),
            "available_funds": num(self.available_funds),
            "currency": self.currency,
        })
    }
}

pub fn account(cfg: &Config) -> Result<Value, AppError> {
    let client = super::connect(cfg)?;
    let account = super::resolve_account(&client, cfg)?;
    let subscription = client
        .account_updates(&account)
        .map_err(|e| AppError::data(format!("account_updates failed: {e}"), "account"))?;

    let mut acc = SummaryAccumulator::default();
    for update in subscription.iter_data() {
        let update = update
            .map_err(|e| AppError::data(format!("account_updates stream: {e}"), "account"))?;
        match update {
            AccountUpdate::AccountValue(v) => acc.absorb(&v.key, v.value, v.currency),
            AccountUpdate::End => break,
            _ => {}
        }
    }

    let mut out = acc.into_summary();
    if let Value::Object(map) = &mut out {
        map.insert("account".to_string(), Value::from(account.0.clone()));
    }
    Ok(out)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absorb_known_keys_and_first_seen_currency() {
        let mut acc = SummaryAccumulator::default();
        acc.absorb("NetLiquidation", "100000.5".into(), "USD".into());
        acc.absorb("TotalCashValue", "25000".into(), "HKD".into());
        acc.absorb("BuyingPower", "200000".into(), String::new());
        acc.absorb("AvailableFunds", "99000".into(), String::new());
        acc.absorb("GrossPositionValue", "1".into(), "EUR".into()); // unknown key: ignored
        let v = acc.into_summary();
        assert_eq!(v["net_liquidation"], json!(100000.5));
        assert_eq!(v["total_cash"], json!(25000.0));
        assert_eq!(v["buying_power"], json!(200000.0));
        assert_eq!(v["available_funds"], json!(99000.0));
        assert_eq!(v["currency"], json!("USD")); // first-seen with NetLiquidation, not HKD/EUR
        assert!(v.get("GrossPositionValue").is_none());
    }

    #[test]
    fn missing_fields_are_null_and_non_numeric_stays_string() {
        let mut acc = SummaryAccumulator::default();
        acc.absorb("NetLiquidation", "n/a".into(), String::new());
        let v = acc.into_summary();
        assert_eq!(v["net_liquidation"], json!("n/a")); // unparseable → raw string
        assert_eq!(v["total_cash"], Value::Null);
        assert_eq!(v["currency"], Value::Null); // empty currency never taken
    }
}
