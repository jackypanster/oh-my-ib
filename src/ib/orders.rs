//! `orders` — open (working) orders as stable JSON. READ ONLY: never places/modifies/
//! cancels. Honors `--account` by filtering rows to the requested account. (card 02)
//! The drain lives in `open_orders_with_client` so `brief` (ADR 0010) shares the SAME
//! logic — including the filter-only-when-`--account`-set semantics (ADR 0011: do NOT
//! auto-filter to the resolved account).

use ibapi::client::blocking::Client;
use ibapi::orders::Orders;
use serde_json::{json, Value};

use crate::config::Config;
use crate::error::AppError;

/// The open-orders row array (no wrapper). Shared by `orders` (own connection) and
/// `brief` (one-session fetch). `account_filter` = the operator's explicit `--account`
/// value, or None (no filtering).
pub(crate) fn open_orders_with_client(
    client: &Client,
    account_filter: Option<&str>,
    ctx: &str,
) -> Result<Value, AppError> {
    let subscription = client
        .all_open_orders()
        .map_err(|e| AppError::data(format!("all_open_orders failed: {e}"), ctx))?;

    let mut out = Vec::new();
    for item in subscription.iter_data() {
        let item = item.map_err(|e| AppError::data(format!("orders stream: {e}"), ctx))?;
        if let Orders::OrderData(d) = item {
            if let Some(acct) = account_filter {
                if d.order.account != acct {
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

    Ok(Value::Array(out))
}

pub fn orders(cfg: &Config) -> Result<Value, AppError> {
    let client = super::connect(cfg)?;
    let open_orders = open_orders_with_client(&client, cfg.account.as_deref(), "orders")?;
    Ok(json!({ "open_orders": open_orders }))
}
