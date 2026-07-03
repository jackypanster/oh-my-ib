//! `brief` — the composite daily snapshot: ONE gateway connection, six sections +
//! `account` + `as_of`, one JSON document. (card brief-command/01)
//!
//! ADR 0010: strictly sequential consume-then-drop fetch, fixed order (resolve → as_of →
//! consolidated drain → pnl take-first → pnl_single sweep → open-orders drain → executions
//! drain). ADR 0011: ONE `account_updates` pass feeds `account_summary` + `positions` + the
//! sweep's discovery list; row/field shaping is the SAME code the six sibling commands run,
//! so sections are byte-shape-identical (PRD criterion 2). Fail-fast, no partial output —
//! the error context names the failing section (`brief/<section>`).

use ibapi::accounts::AccountUpdate;
use serde_json::{json, Value};

use crate::config::Config;
use crate::error::AppError;

use super::account::SummaryAccumulator;
use super::executions::executions_with_client;
use super::orders::open_orders_with_client;
use super::pnl::pnl_with_client;
use super::pnl_by_position::sweep_pnl_singles;
use super::positions::position_row;
use super::shape_pnl_by_position;

/// The pure assembly seam (FROZEN via `tests/brief_command.rs`): exactly the 8 top-level
/// keys, every argument passed through unmodified — no re-shaping, no key invention.
#[allow(clippy::too_many_arguments)] // the 8-key contract IS the signature (frozen spec)
pub fn assemble_brief(
    account: &str,
    as_of: &str,
    account_summary: Value,
    pnl: Value,
    pnl_by_position: Value,
    positions: Value,
    orders: Value,
    executions: Value,
) -> Value {
    json!({
        "account": account,
        "as_of": as_of,
        "account_summary": account_summary,
        "pnl": pnl,
        "pnl_by_position": pnl_by_position,
        "positions": positions,
        "orders": orders,
        "executions": executions,
    })
}

pub fn brief(cfg: &Config) -> Result<Value, AppError> {
    let client = super::connect(cfg)?;
    let account = super::resolve_account(&client, cfg)?;

    // as_of — server truth, ISO-8601 (ADR 0011 §3: `server_time` is UTC by construction —
    // built from a unix timestamp; format via inherent accessors, no time-crate type named).
    let t = client
        .server_time()
        .map_err(|e| AppError::data(format!("server_time failed: {e}"), "brief/as_of"))?;
    let as_of = format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        t.year(),
        u8::from(t.month()),
        t.day(),
        t.hour(),
        t.minute(),
        t.second()
    );

    // Consolidated drain (ADR 0011): ONE account_updates pass feeds three sections.
    let subscription = client.account_updates(&account).map_err(|e| {
        AppError::data(
            format!("account_updates failed: {e}"),
            "brief/account_summary",
        )
    })?;
    let mut summary = SummaryAccumulator::default();
    let mut position_rows: Vec<Value> = Vec::new();
    let mut discovery: Vec<(i32, String)> = Vec::new();
    for update in subscription.iter_data() {
        let update = update.map_err(|e| {
            AppError::data(
                format!("account_updates stream: {e}"),
                "brief/account_summary",
            )
        })?;
        match update {
            AccountUpdate::AccountValue(v) => summary.absorb(&v.key, v.value, v.currency),
            AccountUpdate::PortfolioValue(p) => {
                discovery.push((p.contract.contract_id, p.contract.symbol.to_string()));
                position_rows.push(position_row(&p));
            }
            AccountUpdate::End => break,
            _ => {}
        }
    }
    // ADR 0010: fully consumed, then dropped BEFORE the next request starts.
    drop(subscription);

    let pnl = pnl_with_client(&client, &account, "brief/pnl")?;
    let sweep = sweep_pnl_singles(&client, &account, &discovery, "brief/pnl_by_position")?;
    let pnl_by_position = shape_pnl_by_position(sweep);
    let orders = open_orders_with_client(&client, cfg.account.as_deref(), "brief/orders")?;
    let executions = executions_with_client(&client, &account, "brief/executions")?;

    Ok(assemble_brief(
        &account.0,
        &as_of,
        summary.into_summary(),
        pnl,
        pnl_by_position,
        Value::Array(position_rows),
        orders,
        executions,
    ))
}
