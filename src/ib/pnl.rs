//! `pnl` — account-level Daily / Unrealized / Realized PnL. (card pnl-command/01)
//!
//! Mirrors `account.rs` (connect → request → first reading → JSON), but reads via `reqPnL`
//! (`client.pnl`). KEY difference (ADR 0007): the PnL subscription is an UNBOUNDED real-time stream
//! with NO `End` marker — so we take exactly ONE reading with `next_data()` and drop the subscription.
//! A drain-to-end loop (the `account`/`quote` pattern) would block forever. The take-first read lives
//! in `pnl_with_client` so `brief` (ADR 0010) shares the SAME logic on its own connection.

use ibapi::accounts::types::AccountId;
use ibapi::client::blocking::Client;
use serde_json::{json, Value};

use crate::config::Config;
use crate::error::AppError;

/// The 3-key account-PnL payload (no `account` wrapper — hoisting lives in the callers).
/// Shared by `pnl` (own connection) and `brief` (one-session fetch, ADR 0010).
pub(crate) fn pnl_with_client(
    client: &Client,
    account: &AccountId,
    ctx: &str,
) -> Result<Value, AppError> {
    let subscription = client
        .pnl(account, None)
        .map_err(|e| AppError::data(format!("pnl request failed: {e}"), ctx))?;

    // Take exactly one reading (ADR 0007 — reqPnL has no End marker; do NOT iterate).
    let reading = match subscription.next_data() {
        Some(Ok(p)) => p,
        Some(Err(e)) => return Err(AppError::data(format!("pnl stream: {e}"), ctx)),
        None => return Err(AppError::data("no PnL reading", ctx)),
    };

    Ok(json!({
        "daily_pnl": pnl_number(Some(reading.daily_pnl)),
        "unrealized_pnl": pnl_number(reading.unrealized_pnl),
        "realized_pnl": pnl_number(reading.realized_pnl),
    }))
}

pub fn pnl(cfg: &Config) -> Result<Value, AppError> {
    let client = super::connect(cfg)?;
    let account = super::resolve_account(&client, cfg)?;
    let mut out = pnl_with_client(&client, &account, "pnl")?;
    if let Value::Object(map) = &mut out {
        map.insert("account".to_string(), Value::from(account.0.clone()));
    }
    Ok(out)
}

/// Map an IB PnL value to clean JSON: a finite, real number stays a number; IBKR's "no value"
/// sentinel (`Double.MAX_VALUE` == `f64::MAX` == 1.7976931348623157e308), any non-finite value,
/// or an absent field renders as JSON `null` — so an agent never reports a sentinel as a P&L.
pub fn pnl_number(raw: Option<f64>) -> Value {
    match raw {
        Some(x) if x.is_finite() && x.abs() != f64::MAX => Value::from(x),
        _ => Value::Null,
    }
}
