//! oh-my-ib — read-only Interactive Brokers CLI library.
//!
//! Pure, testable layers (cli/config/output/error) split from the gateway-dependent
//! `ib` layer so the freeze gate can protect the black-box CLI contract.

pub mod cli;
pub mod config;
pub mod error;
pub mod ib;
pub mod output;
pub mod tz;
