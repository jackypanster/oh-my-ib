//! `executions` — account current-day executions (fills), joined to their commission
//! reports by `exec_id`. READ ONLY. Drains to End (ADR 0008 — `ExecutionDataEnd` maps to
//! `EndOfStream`, the orders/positions shape), NOT the reqPnL take-first of ADR 0007.

use std::collections::HashMap;

use ibapi::orders::{Executions, ExecutionFilter};
use serde_json::{json, Value};

use crate::config::Config;
use crate::error::AppError;

use super::pnl_number;

/// Plain, ibapi-free execution row (frozen test constructs these directly).
pub struct ExecRow {
    pub exec_id: String,
    pub order_id: i32,
    pub perm_id: i64,
    pub time: String,
    pub symbol: String,
    pub conid: i32,
    pub side: String,
    pub shares: f64,
    pub price: f64,
    pub cumulative_qty: f64,
    pub avg_price: f64,
    pub exchange: String,
}

/// Plain, ibapi-free commission row (frozen test constructs these directly).
pub struct CommissionRow {
    pub exec_id: String,
    pub commission: f64,
    pub currency: String,
    pub realized_pnl: Option<f64>,
}

/// The pure JOIN seam: exec rows in order, each augmented with its matching commission
/// (by `exec_id`) or `null` commission fields when unmatched. Orphan commissions (no
/// matching exec) are dropped, not emitted as phantom rows.
pub fn merge_executions(execs: Vec<ExecRow>, comms: Vec<CommissionRow>) -> Value {
    let mut by_exec_id: HashMap<String, &CommissionRow> = HashMap::new();
    for c in &comms {
        by_exec_id.insert(c.exec_id.clone(), c);
    }

    let rows: Vec<Value> = execs
        .into_iter()
        .map(|e| {
            let (commission, commission_currency, realized_pnl) = match by_exec_id.get(&e.exec_id)
            {
                Some(c) => (
                    Value::from(c.commission),
                    Value::from(c.currency.clone()),
                    pnl_number(c.realized_pnl),
                ),
                None => (Value::Null, Value::Null, Value::Null),
            };
            json!({
                "exec_id": e.exec_id,
                "order_id": e.order_id,
                "perm_id": e.perm_id,
                "time": e.time,
                "symbol": e.symbol,
                "conid": e.conid,
                "side": e.side,
                "shares": e.shares,
                "price": e.price,
                "cumulative_qty": e.cumulative_qty,
                "avg_price": e.avg_price,
                "exchange": e.exchange,
                "commission": commission,
                "commission_currency": commission_currency,
                "realized_pnl": realized_pnl,
            })
        })
        .collect();

    Value::Array(rows)
}

pub fn executions(cfg: &Config) -> Result<Value, AppError> {
    let client = super::connect(cfg)?;
    let account = super::resolve_account(&client, cfg)?;

    let filter = ExecutionFilter {
        account_code: account.0.clone(),
        ..Default::default()
    };
    let subscription = client
        .executions(filter)
        .map_err(|e| AppError::data(format!("executions request failed: {e}"), "executions"))?;

    let mut execs = Vec::new();
    let mut comms = Vec::new();
    for item in subscription.iter_data() {
        let item = item.map_err(|e| AppError::data(format!("executions stream: {e}"), "executions"))?;
        match item {
            Executions::ExecutionData(d) => {
                execs.push(ExecRow {
                    exec_id: d.execution.execution_id,
                    order_id: d.execution.order_id,
                    perm_id: d.execution.perm_id,
                    time: d.execution.time,
                    symbol: d.contract.symbol.to_string(),
                    conid: d.contract.contract_id,
                    side: d.execution.side.as_str().to_string(),
                    shares: d.execution.shares,
                    price: d.execution.price,
                    cumulative_qty: d.execution.cumulative_quantity,
                    avg_price: d.execution.average_price,
                    exchange: d.execution.exchange,
                });
            }
            Executions::CommissionReport(c) => {
                comms.push(CommissionRow {
                    exec_id: c.execution_id,
                    commission: c.commission,
                    currency: c.currency,
                    realized_pnl: c.realized_pnl,
                });
            }
        }
    }

    Ok(json!({ "account": account.0, "executions": merge_executions(execs, comms) }))
}
