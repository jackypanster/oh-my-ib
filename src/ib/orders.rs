//! `orders` — open (working) orders. READ ONLY: never places/modifies/cancels. (card 02)

use serde_json::json;

use crate::config::Config;
use crate::error::AppError;

pub fn orders(cfg: &Config) -> Result<serde_json::Value, AppError> {
    let client = super::connect(cfg)?;
    let subscription = client
        .all_open_orders()
        .map_err(|e| AppError::data(format!("all_open_orders failed: {e}"), "orders"))?;

    // The `Orders` stream yields OrderData / OrderStatus items. Field-level JSON
    // shaping is refined after live paper-account verification (Phase 1 acceptance);
    // the first cut preserves the full item via its debug form.
    let mut out = Vec::new();
    for item in subscription.iter_data() {
        let item = item.map_err(|e| AppError::data(format!("orders stream: {e}"), "orders"))?;
        out.push(json!(format!("{item:?}")));
    }
    Ok(json!({ "open_orders": out }))
}
