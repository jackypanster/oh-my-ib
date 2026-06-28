//! The gateway-dependent layer: connect to IB Gateway and run read-only requests.
//! NOT covered by the frozen spec (needs a live gateway) — reviewed by reading +
//! manual paper-account acceptance. Every ibapi error is mapped to an AppError.

use ibapi::client::blocking::Client;

use crate::config::Config;
use crate::error::AppError;

mod account;
mod client;
mod contract;
mod history;
mod orders;
mod positions;
mod quote;

pub use account::account;
pub use client::health;
pub use contract::contract;
pub use history::history;
pub use orders::orders;
pub use positions::positions;
pub use quote::quote;

/// Connect to the IB Gateway. A dead/absent gateway yields an AppError::Connection
/// (the offline-deterministic path the frozen tests assert against a dead port).
pub(crate) fn connect(cfg: &Config) -> Result<Client, AppError> {
    Client::connect(&cfg.address(), cfg.client_id).map_err(|e| {
        AppError::connection(format!("cannot connect to IB Gateway: {e}"), cfg.address())
    })
}
