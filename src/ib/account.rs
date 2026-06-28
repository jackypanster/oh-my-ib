//! `account` — account summary (net liq, cash, buying power, available funds). (card 02)

use ibapi::accounts::types::AccountGroup;
use ibapi::accounts::{AccountSummaryResult, AccountSummaryTags};
use serde_json::json;

use crate::config::Config;
use crate::error::AppError;

pub fn account(cfg: &Config) -> Result<serde_json::Value, AppError> {
    let client = super::connect(cfg)?;
    let tags = &[
        AccountSummaryTags::NET_LIQUIDATION,
        AccountSummaryTags::TOTAL_CASH_VALUE,
        AccountSummaryTags::BUYING_POWER,
        AccountSummaryTags::AVAILABLE_FUNDS,
    ];
    let subscription = client
        .account_summary(&AccountGroup("All".to_string()), tags)
        .map_err(|e| AppError::data(format!("account_summary failed: {e}"), "account"))?;

    let mut summary = serde_json::Map::new();
    for update in subscription.iter_data() {
        let update = update
            .map_err(|e| AppError::data(format!("account_summary stream: {e}"), "account"))?;
        match update {
            AccountSummaryResult::Summary(s) => {
                summary.insert(
                    s.tag.to_string(),
                    json!({ "value": s.value, "currency": s.currency, "account": s.account }),
                );
            }
            AccountSummaryResult::End => {
                subscription.cancel();
                break;
            }
        }
    }
    Ok(json!({ "account_summary": summary }))
}
