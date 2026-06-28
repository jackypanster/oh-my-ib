//! `positions` — current positions. (card 02)

use ibapi::accounts::PositionUpdate;
use serde_json::json;

use crate::config::Config;
use crate::error::AppError;

pub fn positions(cfg: &Config) -> Result<serde_json::Value, AppError> {
    let client = super::connect(cfg)?;
    let subscription = client
        .positions()
        .map_err(|e| AppError::data(format!("positions failed: {e}"), "positions"))?;

    let mut out = Vec::new();
    while let Some(update) = subscription.next_data() {
        let update =
            update.map_err(|e| AppError::data(format!("positions stream: {e}"), "positions"))?;
        match update {
            PositionUpdate::Position(p) => {
                out.push(json!({
                    "symbol": p.contract.symbol,
                    "conid": p.contract.contract_id,
                    "position": p.position,
                    "average_cost": p.average_cost,
                }));
            }
            PositionUpdate::PositionEnd => {
                subscription.cancel();
                break;
            }
        }
    }
    Ok(json!({ "positions": out }))
}
