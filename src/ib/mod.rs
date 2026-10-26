//! The gateway-dependent layer: connect to IB Gateway and run read-only requests.
//! NOT covered by the frozen spec (needs a live gateway) — reviewed by reading +
//! manual paper-account acceptance. Every ibapi error is mapped to an AppError.

use std::time::Duration;

use ibapi::accounts::types::AccountId;
use ibapi::client::blocking::Client;

use crate::config::Config;
use crate::error::AppError;

mod account;
mod ledger;
mod brief;
mod client;
mod completed_orders;
mod contract;
mod executions;
mod history;
mod orders;
mod grid;
mod sma_tick;
mod option_chain;
mod option_quote;
mod delta_legs;
mod pcs_scan;
mod pnl;
mod spreads;
mod pnl_by_position;
mod positions;
mod quote;
mod signal;
mod search;
mod trade;

pub use account::account;
pub use ledger::ledger;
pub use grid::grid_tick;
pub use brief::{assemble_brief, brief};
pub use client::health;
pub use contract::contract;
pub use executions::{executions, merge_executions, CommissionRow, ExecRow};
pub use history::history;
pub use orders::orders;
pub use pnl::{pnl, pnl_number};
pub use pnl_by_position::{pnl_by_position, shape_pnl_by_position, PnlSingleRow};
pub use positions::{position_row, positions};
pub use option_chain::{filter_chain_rows, option_chain, shape_option_chain, ChainRow};
pub use option_quote::{option_quote, option_quote_greeks, shape_option_quote, GreeksRow};
pub use delta_legs::{
    delta_legs, select_delta_legs, shape_delta_legs, validate_delta_legs_params, DeltaLegsParams,
    LegCandidate, LegKind, LegRow, DELTA_MAX, DELTA_MIN, FLOOR, SORT_AIM, TARGET_MAX, TARGET_MIN,
};
pub use spreads::{shape_spreads, spreads, SpreadsRow};
pub use pcs_scan::{
    coarse_strikes, estimate_target_strike, pcs_scan, pick_expiry, pick_spot, refine_strikes,
    select_pcs, shape_pcs_scan, validate_params, PcsCandidate, PcsParams, PutRow, SkipReason,
    PCS_BATCH_LINES,
};
pub use quote::{quote, quote_price_tick, shape_quotes};
pub use search::{search, shape_search, SearchRow};
pub use completed_orders::{completed_orders, shape_completed_orders, CompletedOrderRow};
pub use trade::{apply_outside_rth, assignment_notional, blocking_close_order_ids, build_combo_order, build_option_order, build_stk_order, cancel, buy, check_coverage, check_short_option_held, covered_call_close, covered_call_open, sell, check_live_write_posture, classify_submit_error, combo_live_max_risk, compute_notional, derive_close, option_buy, option_close, option_combo, option_sell, parse_combo_leg, plan_reprice, refuse_live_combo_on_live, reprice, reprice_order, reprice_transmits, require_live_write_gate, resolve_max_notional, shape_cancel_preview, shape_combo_order_ack, shape_option_close_ack, shape_order_ack, shape_option_order_ack, shape_preview, shape_reprice_preview, short_put_close, short_put_open, stamp_order_account, submit_timeout_error, write_transmits, ConfirmedReprice, Coverage, GuardRow, LegSpec, RepricePlan, RepriceTarget};
pub use signal::{sma_signal, sma_signal_cmd, Bar, SmaSignal, SignalState};
pub use sma_tick::{plan_sma_tick, sma_tick_cmd, TickAction};

const MAX_CONNECT_RETRIES: u32 = 3;
const CONNECT_BACKOFF_MS: u64 = 250;

/// Shared bound for take-first reads on markerless streams (ADR 0012): `reqPnL` /
/// `reqPnLSingle` have no `End` marker, so we read exactly one item within this window instead
/// of blocking forever on a wedged PnL channel. Fixed (not configurable — PRD D3): the wedge
/// is silent (emits nothing), so tunability buys nothing; a healthy first tick arrives <1s live.
pub const TAKE_FIRST_TIMEOUT: Duration = Duration::from_secs(10);

/// Total per-snapshot deadline (ADR 0038): ONE bound for the WHOLE snapshot drain, measured
/// from just after `subscribe()` — not a per-item window (`timeout_iter_data` resets on every
/// tick, so a trickle could extend the drain unboundedly). 20s > IB's doc-cited ≈11s
/// `tickSnapshotEnd` tail, so a healthy snapshot never false-times-out; deliberately NOT
/// `TAKE_FIRST_TIMEOUT` (10s < the 11s tail). Live-proven need: a silent snapshot (no tick,
/// no error, no SnapshotEnd) otherwise blocks forever.
pub const SNAPSHOT_DEADLINE: Duration = Duration::from_secs(20);

/// Pure builder for the snapshot-drain timeout error (ADR 0038 §Decision 3): code `timeout`,
/// exit 6. `instrument` is the symbol (quote) or `<symbol> <expiry> <strike> <C|P>`
/// (option-quote); `md_type` is the CLI spelling `live|delayed|frozen`; `context` is
/// `quote/<symbol>` / `option-quote`. Single-line message; the `20` is formatted from
/// `SNAPSHOT_DEADLINE.as_secs()`, never a literal.
pub fn snapshot_timeout_error(instrument: &str, md_type: &str, context: &str) -> AppError {
    AppError::timeout(
        format!(
            "no SnapshotEnd within {}s for {instrument} (md-type={md_type}) — no real-time snapshot entitlement for this instrument, or market closed with no ticks; try --md-type delayed",
            SNAPSHOT_DEADLINE.as_secs()
        ),
        context,
    )
}

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

/// Soft/advisory IB notice codes that must not fail `managed_accounts`.
///
/// ibapi classifies warnings as 2100..=2169 (`Notice::is_warning`). Tiger/IB
/// client-version advisory **2172** (soft until 2026-12-15: upgrade from 1045.1
/// toward 1050.1) sits just outside that range, so `one_shot` surfaces it as
/// `Err(Error::Notice)` and aborts before the ManagedAccounts payload. Codes in
/// 2100..=2199 are treated as soft for this read path only.
pub fn is_soft_managed_accounts_notice(code: i32) -> bool {
    (2100..=2199).contains(&code)
}

const MANAGED_ACCOUNTS_SOFT_ATTEMPTS: u32 = 3;

/// Soft-notice retry loop for `managed_accounts`, injectable for tests.
///
/// Calls `fetch` up to [`MANAGED_ACCOUNTS_SOFT_ATTEMPTS`] times. Soft notices
/// (see [`is_soft_managed_accounts_notice`]) accumulate as Display strings in
/// `warnings` and retry; any other error fails immediately. Returns
/// `(accounts, warnings)` on success. Does not touch the write path.
pub fn managed_accounts_tolerant_with<F>(
    mut fetch: F,
) -> Result<(Vec<String>, Vec<String>), AppError>
where
    F: FnMut() -> Result<Vec<String>, ibapi::Error>,
{
    let mut warnings = Vec::new();
    for attempt in 1..=MANAGED_ACCOUNTS_SOFT_ATTEMPTS {
        match fetch() {
            Ok(accounts) => return Ok((accounts, warnings)),
            Err(ibapi::Error::Notice(n)) if is_soft_managed_accounts_notice(n.code) => {
                warnings.push(n.to_string());
                if attempt == MANAGED_ACCOUNTS_SOFT_ATTEMPTS {
                    return Err(AppError::data(
                        format!(
                            "managed_accounts only returned soft notices after {MANAGED_ACCOUNTS_SOFT_ATTEMPTS} attempts: {}",
                            warnings.join("; ")
                        ),
                        "managed_accounts",
                    ));
                }
            }
            Err(e) => {
                return Err(AppError::data(
                    format!("managed_accounts failed: {e}"),
                    "managed_accounts",
                ));
            }
        }
    }
    unreachable!("managed_accounts_tolerant_with loop always returns")
}

/// `Client::managed_accounts` with soft-notice tolerance.
///
/// Thin wrapper over [`managed_accounts_tolerant_with`]. Retries when the
/// gateway emits a soft notice (e.g. 2172) ahead of the ManagedAccounts payload.
pub(crate) fn managed_accounts_tolerant(
    client: &Client,
) -> Result<(Vec<String>, Vec<String>), AppError> {
    managed_accounts_tolerant_with(|| client.managed_accounts())
}

/// Resolve the account to scope a request to: the configured/`--account` value if set,
/// else the first managed account. Errors if none is available.
///
/// When `cfg.account` is set, short-circuits without calling the gateway (unchanged).
/// Otherwise uses [`managed_accounts_tolerant`] so soft notice 2172 cannot block
/// account resolution.
pub(crate) fn resolve_account(client: &Client, cfg: &Config) -> Result<AccountId, AppError> {
    if let Some(account) = &cfg.account {
        return Ok(AccountId(account.clone()));
    }
    let (accounts, _warnings) = managed_accounts_tolerant(client)
        .map_err(|e| AppError::data(e.message, "resolve account"))?;
    accounts
        .into_iter()
        .next()
        .map(AccountId)
        .ok_or_else(|| AppError::not_found("no managed account available", "resolve account"))
}
