//! `pnl_by_position` — per-position Daily / Unrealized / Realized PnL. (card pnl-by-position/01)
//!
//! Two-phase sweep (ADR 0009): discovery via the `account_updates` portfolio stream (the
//! `positions.rs` pattern — drain to `End`, collect conid + symbol), then ONE `pnl_single`
//! take-first read per conid (ADR 0007 — reqPnLSingle is markerless, a drain loop would hang
//! forever). Fail-fast: any failed read aborts the whole command; a partial sweep is
//! indistinguishable from a complete one to the consuming agent.

use ibapi::accounts::types::ContractId;
use ibapi::accounts::AccountUpdate;
use serde_json::{json, Value};

use crate::config::Config;
use crate::error::AppError;

use super::pnl_number;

/// Plain, ibapi-free per-position row (frozen test constructs these directly).
pub struct PnlSingleRow {
    pub conid: i32,
    pub symbol: String,
    pub position: f64,
    pub daily_pnl: f64,
    pub unrealized_pnl: f64,
    pub realized_pnl: f64,
    pub value: f64,
}

/// The pure shaping seam: rows in order → JSON array. Money fields (`daily_pnl`,
/// `unrealized_pnl`, `realized_pnl`, and defensively `value`) route through `pnl_number`
/// (IBKR unset sentinel / non-finite → `null` — `PnLSingle` fields are bare `f64`, so the
/// sentinel arrives as a value). Identity fields (`conid`, `symbol`, `position`) pass through
/// raw — a position of 0 is data (closed-today row), never filtered.
pub fn shape_pnl_by_position(rows: Vec<PnlSingleRow>) -> Value {
    let out: Vec<Value> = rows
        .into_iter()
        .map(|r| {
            json!({
                "conid": r.conid,
                "symbol": r.symbol,
                "position": r.position,
                "daily_pnl": pnl_number(Some(r.daily_pnl)),
                "unrealized_pnl": pnl_number(Some(r.unrealized_pnl)),
                "realized_pnl": pnl_number(Some(r.realized_pnl)),
                "value": pnl_number(Some(r.value)),
            })
        })
        .collect();
    Value::Array(out)
}

pub fn pnl_by_position(cfg: &Config) -> Result<Value, AppError> {
    let client = super::connect(cfg)?;
    let account = super::resolve_account(&client, cfg)?;

    // Phase 1 — discovery (positions.rs pattern): drain the portfolio stream to End,
    // collecting (conid, symbol) for every position, qty==0 included (PRD D6).
    let subscription = client
        .account_updates(&account)
        .map_err(|e| AppError::data(format!("account_updates failed: {e}"), "pnl-by-position"))?;
    let mut contracts: Vec<(i32, String)> = Vec::new();
    for update in subscription.iter_data() {
        let update = update.map_err(|e| {
            AppError::data(format!("account_updates stream: {e}"), "pnl-by-position")
        })?;
        match update {
            AccountUpdate::PortfolioValue(p) => {
                contracts.push((p.contract.contract_id, p.contract.symbol.to_string()));
            }
            AccountUpdate::End => break,
            _ => {}
        }
    }
    // Unsubscribe before the sweep (Drop sends the cancel; ADR 0009 phase boundary).
    drop(subscription);

    // Phase 2 — sweep: one take-first read per conid, in discovery order (ADR 0009).
    // `symbol` comes from discovery (PnLSingle carries no contract identity); position/value/PnL
    // come from the reading (fresher than the portfolio snapshot).
    let mut rows = Vec::with_capacity(contracts.len());
    for (conid, symbol) in contracts {
        let sub = client
            .pnl_single(&account, ContractId::from(conid), None)
            .map_err(|e| {
                AppError::data(
                    format!("pnl_single conid {conid}: request failed: {e}"),
                    "pnl-by-position",
                )
            })?;
        // Take exactly one reading (ADR 0007/0009 — markerless stream; do NOT iterate).
        let reading = match sub.next_data() {
            Some(Ok(p)) => p,
            Some(Err(e)) => {
                return Err(AppError::data(
                    format!("pnl_single conid {conid}: stream: {e}"),
                    "pnl-by-position",
                ))
            }
            None => {
                return Err(AppError::data(
                    format!("pnl_single conid {conid}: no PnL reading"),
                    "pnl-by-position",
                ))
            }
        };
        rows.push(PnlSingleRow {
            conid,
            symbol,
            position: reading.position,
            daily_pnl: reading.daily_pnl,
            unrealized_pnl: reading.unrealized_pnl,
            realized_pnl: reading.realized_pnl,
            value: reading.value,
        });
    }

    Ok(json!({ "account": account.0, "by_position": shape_pnl_by_position(rows) }))
}
