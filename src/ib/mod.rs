//! The gateway-dependent layer: connect to IB Gateway and run read-only requests.
//! NOT covered by the frozen spec (needs a live gateway) — reviewed by reading +
//! manual paper-account acceptance. Every ibapi error is mapped to an AppError.

use std::time::Duration;

use ibapi::accounts::types::AccountId;
use ibapi::client::blocking::Client;

use crate::config::Config;
use crate::error::AppError;

mod account;
mod brief;
mod client;
mod completed_orders;
mod contract;
mod executions;
mod history;
mod orders;
mod pnl;
mod pnl_by_position;
mod positions;
mod quote;
mod search;
mod trade;

pub use account::account;
pub use brief::{assemble_brief, brief};
pub use client::health;
pub use contract::contract;
pub use executions::{executions, merge_executions, CommissionRow, ExecRow};
pub use history::history;
pub use orders::orders;
pub use pnl::{pnl, pnl_number};
pub use pnl_by_position::{pnl_by_position, shape_pnl_by_position, PnlSingleRow};
pub use positions::positions;
pub use quote::{quote, quote_price_tick, shape_quotes};
pub use search::{search, shape_search, SearchRow};
pub use completed_orders::{completed_orders, shape_completed_orders, CompletedOrderRow};
pub use trade::{build_stk_order, cancel, buy, sell, shape_order_ack, require_live_write_gate};

const MAX_CONNECT_RETRIES: u32 = 3;
const CONNECT_BACKOFF_MS: u64 = 250;

/// Shared bound for take-first reads on markerless streams (ADR 0012): `reqPnL` /
/// `reqPnLSingle` have no `End` marker, so we read exactly one item within this window instead
/// of blocking forever on a wedged PnL channel. Fixed (not configurable — PRD D3): the wedge
/// is silent (emits nothing), so tunability buys nothing; a healthy first tick arrives <1s live.
pub const TAKE_FIRST_TIMEOUT: Duration = Duration::from_secs(10);

/// Whether a connection error is transient (worth a short retry) vs permanent. EAGAIN maps to
/// `WouldBlock` — the error seen when two account-scoped commands reconnect back-to-back with the
/// same client_id before the gateway has released the prior subscription.
pub fn is_transient_io(kind: std::io::ErrorKind) -> bool {
    matches!(
        kind,
        std::io::ErrorKind::WouldBlock
            | std::io::ErrorKind::Interrupted
            | std::io::ErrorKind::TimedOut
    )
}

/// Connect to the IB Gateway. A dead/absent gateway yields an AppError::Connection
/// (the offline-deterministic path the frozen tests assert against a dead port). Transient
/// errors (EAGAIN/WouldBlock) are retried a few times with a short backoff; permanent errors
/// (e.g. connection refused) fail fast.
pub(crate) fn connect(cfg: &Config) -> Result<Client, AppError> {
    // Teach ibapi the gateway timezone abbreviations it doesn't know (e.g. HKT) BEFORE connecting,
    // so connection works without the IBAPI_TIMEZONE_ALIASES env var.
    crate::tz::register_builtin_aliases();
    let address = cfg.address();
    let mut attempt: u32 = 0;
    loop {
        match Client::connect(&address, cfg.client_id) {
            Ok(client) => return Ok(client),
            Err(err) => {
                let transient = matches!(&err, ibapi::Error::Io(e) if is_transient_io(e.kind()));
                if transient && attempt < MAX_CONNECT_RETRIES {
                    attempt += 1;
                    std::thread::sleep(Duration::from_millis(CONNECT_BACKOFF_MS * attempt as u64));
                    continue;
                }
                return Err(AppError::connection(
                    format!("cannot connect to IB Gateway: {err}"),
                    address,
                ));
            }
        }
    }
}

/// Resolve the account to scope a request to: the configured/`--account` value if set,
/// else the first managed account. Errors if none is available.
pub(crate) fn resolve_account(client: &Client, cfg: &Config) -> Result<AccountId, AppError> {
    if let Some(account) = &cfg.account {
        return Ok(AccountId(account.clone()));
    }
    let accounts = client
        .managed_accounts()
        .map_err(|e| AppError::data(format!("managed_accounts failed: {e}"), "resolve account"))?;
    accounts
        .into_iter()
        .next()
        .map(AccountId)
        .ok_or_else(|| AppError::not_found("no managed account available", "resolve account"))
}
