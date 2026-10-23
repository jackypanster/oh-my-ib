//! `health` — connect and report gateway/connection status. (card 01)

use serde_json::json;

use crate::config::Config;
use crate::error::AppError;

pub fn health(cfg: &Config) -> Result<serde_json::Value, AppError> {
    let client = super::connect(cfg)?;
    let (accounts, warnings) = super::managed_accounts_tolerant(&client)
        .map_err(|e| AppError::data(e.message, "health"))?;
    let server_time = client.server_time().ok().map(|t| format!("{t:?}"));
    Ok(json!({
        "connected": true,
        "address": cfg.address(),
        "server_version": client.server_version(),
        "accounts": accounts,
        "server_time": server_time,
        "warnings": warnings,
    }))
}
