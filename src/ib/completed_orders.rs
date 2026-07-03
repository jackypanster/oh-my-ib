//! `completed-orders` — today's terminal-state orders (filled/cancelled/rejected) as stable
//! JSON. READ ONLY: never places/modifies/cancels (Phase 1 read-only red line). Mirrors the
//! `orders` sibling's drain and filter-when-set semantics. (card 01)
//!
//! Drain class: `completed_orders(false)` returns a `Subscription<Orders>` on a shared channel
//! whose response set `[CompletedOrder, CompletedOrdersEnd]` self-terminates `iter_data()` —
//! the `orders.rs` drain-to-End pattern verbatim, NOT ADR 0012's markerless take-first class
//! (no `TAKE_FIRST_TIMEOUT`). `api_only=false` is hardcoded (operator trades via the Tiger
//! app; an API-only view is empty by construction — ADR 0015 D4).

use ibapi::orders::Orders;
use serde_json::{json, Value};

use crate::config::Config;
use crate::error::AppError;

/// Plain, ibapi-free completed-order row — the frozen test constructs these directly (the
/// `SearchRow`/`PnlSingleRow` pattern), so the pure shaping seam is offline-testable with no
/// gateway. 14 keys: open-orders 10-key parity + 4 completion keys (`status`,
/// `filled_quantity`, `completed_time`, `completed_status`).
pub struct CompletedOrderRow {
    pub order_id: i32,
    pub account: String,
    pub symbol: String,
    pub conid: i32,
    pub action: String,
    pub quantity: f64,
    pub order_type: String,
    pub limit_price: Option<f64>,
    pub aux_price: Option<f64>,
    pub tif: String,
    pub status: String,
    pub filled_quantity: f64,
    pub completed_time: String,
    pub completed_status: String,
}

/// The pure, FROZEN seam: rows in gateway order → a JSON array of exact 14-key objects.
/// `limit_price`/`aux_price` pass through raw (`None` → `null`, present keys — NOT the
/// `pnl_number` sentinel, which is for money `f64` only). Empty ⇒ `json!([])`.
pub fn shape_completed_orders(rows: Vec<CompletedOrderRow>) -> Value {
    Value::Array(
        rows.into_iter()
            .map(|r| {
                json!({
                    "order_id": r.order_id,
                    "account": r.account,
                    "symbol": r.symbol,
                    "conid": r.conid,
                    "action": r.action,
                    "quantity": r.quantity,
                    "order_type": r.order_type,
                    "limit_price": r.limit_price,
                    "aux_price": r.aux_price,
                    "tif": r.tif,
                    "status": r.status,
                    "filled_quantity": r.filled_quantity,
                    "completed_time": r.completed_time,
                    "completed_status": r.completed_status,
                })
            })
            .collect(),
    )
}

/// Read-only drain of today's completed orders: connect → `completed_orders(false)` →
/// `iter_data()` to the natural End marker → shape. `Orders::OrderData` arm only
/// (`OrderStatus` variants skipped, the `orders` posture). Rows are filtered to the explicit
/// `--account` value ONLY when set (ADR 0011 / ADR 0015 D5: never auto-filter to the resolved
/// account).
pub fn completed_orders(cfg: &Config) -> Result<Value, AppError> {
    let client = super::connect(cfg)?;
    let subscription = client
        .completed_orders(false)
        .map_err(|e| AppError::data(format!("completed_orders failed: {e}"), "completed-orders"))?;
    let mut rows = Vec::new();
    for item in subscription.iter_data() {
        let item = item.map_err(|e| AppError::data(format!("completed orders stream: {e}"), "completed-orders"))?;
        if let Orders::OrderData(d) = item {
            if let Some(acct) = cfg.account.as_deref() {
                if d.order.account != acct {
                    continue;
                }
            }
            rows.push(CompletedOrderRow {
                order_id: d.order_id,
                account: d.order.account,
                symbol: d.contract.symbol.to_string(),
                conid: d.contract.contract_id,
                action: format!("{:?}", d.order.action),
                quantity: d.order.total_quantity,
                order_type: d.order.order_type,
                limit_price: d.order.limit_price,
                aux_price: d.order.aux_price,
                tif: format!("{:?}", d.order.tif),
                status: format!("{:?}", d.order_state.status),
                filled_quantity: d.order.filled_quantity,
                completed_time: d.order_state.completed_time,
                completed_status: d.order_state.completed_status,
            });
        }
    }
    Ok(json!({ "completed_orders": shape_completed_orders(rows) }))
}
