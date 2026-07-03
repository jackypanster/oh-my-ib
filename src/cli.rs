//! clap command-line surface. Global options propagate to every subcommand so
//! they parse before AND after the subcommand token (the CLI contract relies on
//! `omi --format json health --host .. --port ..`).

use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Parser, Debug)]
#[command(
    name = "omi",
    version,
    about = "Read-only Interactive Brokers CLI (TWS API via ibapi)"
)]
pub struct Cli {
    #[command(flatten)]
    pub global: GlobalOpts,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Args, Debug, Default)]
pub struct GlobalOpts {
    /// Output format (default: json)
    #[arg(long, global = true, value_enum)]
    pub format: Option<Format>,
    /// Gateway host (default: 127.0.0.1)
    #[arg(long, global = true)]
    pub host: Option<String>,
    /// Gateway port (default: 4002 paper)
    #[arg(long, global = true)]
    pub port: Option<u16>,
    /// Use the LIVE account (port 4001) instead of paper
    #[arg(long, global = true)]
    pub live: bool,
    /// API client id (default: 100)
    #[arg(long, global = true)]
    pub client_id: Option<i32>,
    /// Account id (default: first managed account)
    #[arg(long, global = true)]
    pub account: Option<String>,
    /// Market data type: live|delayed|frozen (default: config / delayed)
    #[arg(long, global = true)]
    pub md_type: Option<String>,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Json,
    Table,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Check the gateway connection and report accounts
    Health,
    /// Daily account snapshot: summary, PnL, positions, orders, executions (one connection)
    Brief,
    /// Account summary (net liq, cash, buying power)
    Account,
    /// Account PnL (daily, unrealized, realized)
    Pnl,
    /// Per-position PnL (daily, unrealized, realized)
    PnlByPosition,
    /// Current positions
    Positions,
    /// Open (working) orders — read only
    Orders,
    /// Current-day executions (fills), joined to commission reports
    Executions,
    /// Snapshot quote for a symbol
    Quote(QuoteArgs),
    /// Resolve contract details for a symbol
    Contract(ContractArgs),
    /// Historical bars for a symbol
    History(HistoryArgs),
}

#[derive(Args, Debug)]
pub struct QuoteArgs {
    /// Ticker symbol(s), e.g. AAPL MSFT
    #[arg(required = true, value_name = "SYMBOL")]
    pub symbols: Vec<String>,
    /// Security type (Phase 1: STK only)
    #[arg(long, default_value = "STK")]
    pub sec_type: String,
    /// Exchange
    #[arg(long, default_value = "SMART")]
    pub exchange: String,
    /// Currency
    #[arg(long, default_value = "USD")]
    pub currency: String,
}

#[derive(Args, Debug)]
pub struct ContractArgs {
    /// Ticker symbol, e.g. AAPL
    pub symbol: String,
    /// Security type
    #[arg(long, default_value = "STK")]
    pub sec_type: String,
}

#[derive(Args, Debug)]
pub struct HistoryArgs {
    /// Ticker symbol, e.g. AAPL
    pub symbol: String,
    /// Bar size, e.g. 1d, 1h, 5m
    #[arg(long, default_value = "1d")]
    pub bar: String,
    /// Duration, e.g. 1M, 2W, 30D, 1Y
    #[arg(long, default_value = "1M")]
    pub duration: String,
}
