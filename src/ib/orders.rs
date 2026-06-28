//! `orders` — open (working) orders as stable JSON. READ ONLY: never places/modifies/
//! cancels. Honors `--account` by filtering rows to the requested account. (card 02)

use ibapi::orders::Orders;
use serde_json::{json, Value};

use crate::config::Config;
use crate::error::AppError;

pub fn orders(cfg: &Config) -> Result<Value, AppError> {
    let client = super::connect(cfg)?;
    let subscription = client
        .all_open_orders()
        .map_err(|e| AppError::data(format!("all_open_orders failed: {e}"), "orders"))?;

    let mut out = Vec::new();
    for item in subscription.iter_data() {
        let item = item.map_err(|e| AppError::data(format!("orders stream: {e}"), "orders"))?;
        if let Orders::OrderData(d) = item {
            if let Some(account) = &cfg.account {
                if &d.order.account != account {
                    continue;
                }
            }
            out.push(json!({
                "order_id": d.order_id,
                "account": d.order.account,
                "symbol": d.contract.symbol,
                "conid": d.contract.contract_id,
                "action": format!("{:?}", d.order.action),
                "quantity": d.order.total_quantity,
                "order_type": d.order.order_type,
                "limit_price": d.order.limit_price,
                "aux_price": d.order.aux_price,
                "tif": format!("{:?}", d.order.tif),
            }));
        }
    }

    Ok(json!({ "open_orders": out }))
}
