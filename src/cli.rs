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
    /// Preview the order via IB whatIf (no transmit): resolved contract + margin/commission,
    /// no order placed.
    #[arg(long, global = true)]
    pub preview: bool,
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
    /// Fuzzy symbol/company search
    Search(SearchArgs),
    /// Today's completed orders (filled/cancelled) with status
    CompletedOrders,
    /// Place a BUY order (paper by default; live needs --live + OMI_ALLOW_LIVE=1)
    Buy(OrderArgs),
    /// Place a SELL order (paper by default; live needs --live + OMI_ALLOW_LIVE=1)
    Sell(OrderArgs),
    /// Cancel an order by id (paper by default; live needs --live + OMI_ALLOW_LIVE=1)
    Cancel(CancelArgs),
    /// Option chain (expirations × strikes) for an underlying
    OptionChain(OptionChainArgs),
    /// Snapshot quote + greeks for one option contract
    OptionQuote(OptionQuoteArgs),
    /// Place a single-leg option BUY (LMT/DAY; paper default; live needs --live + OMI_ALLOW_LIVE=1)
    OptionBuy(OptionOrderArgs),
    /// Place a single-leg option SELL (LMT/DAY; paper default; live needs --live + OMI_ALLOW_LIVE=1)
    OptionSell(OptionOrderArgs),
    /// Place a multi-leg option combo (BAG, LMT/DAY; paper default; live needs --live + OMI_ALLOW_LIVE=1)
    OptionCombo(OptionComboArgs),
    /// Close a HELD option position by conid (side derived from held position; LMT/DAY; paper default; live needs --live + OMI_ALLOW_LIVE=1)
    OptionClose(OptionCloseArgs),
}

#[derive(Args, Debug)]
pub struct OrderArgs {
    /// Ticker symbol, e.g. AAPL
    pub symbol: String,
    /// Quantity (positive)
    pub quantity: f64,
    /// Limit price (omit for MKT; positive for LMT)
    #[arg(long)]
    pub limit: Option<f64>,
}

#[derive(Args, Debug)]
pub struct CancelArgs {
    /// Order id to cancel
    pub order_id: i32,
}

#[derive(Args, Debug)]
pub struct SearchArgs {
    /// Search text, e.g. apple or "hong kong"
    pub pattern: String,
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

/// omi option-chain AAPL [--exchange SMART]
#[derive(Args, Debug)]
pub struct OptionChainArgs {
    /// Underlying ticker symbol, e.g. AAPL
    pub symbol: String,
    /// Client-side exchange filter; `SMART` (default) = consolidated view, `''` = all exchanges
    #[arg(long, default_value = "SMART")]
    pub exchange: String,
}

/// omi option-quote --symbol AAPL --expiry 20260918 --strike 250 --right C
#[derive(Args, Debug)]
pub struct OptionQuoteArgs {
    /// Underlying ticker symbol, e.g. AAPL
    #[arg(long)]
    pub symbol: String,
    /// Expiry date, 8-digit YYYYMMDD, e.g. 20260918
    #[arg(long)]
    pub expiry: String,
    /// Strike price (> 0)
    #[arg(long)]
    pub strike: f64,
    /// Right: C|CALL or P|PUT (case-insensitive)
    #[arg(long)]
    pub right: String,
    /// Exchange
    #[arg(long, default_value = "SMART")]
    pub exchange: String,
    /// Currency
    #[arg(long, default_value = "USD")]
    pub currency: String,
    /// Trading class, e.g. SPXW (optional; gateway resolves when absent)
    #[arg(long)]
    pub trading_class: Option<String>,
}

/// omi option-buy --symbol AAPL --expiry 20260918 --strike 250 --right C --qty 1 --limit 5.50
#[derive(Args, Debug)]
pub struct OptionOrderArgs {
    /// Underlying ticker symbol, e.g. AAPL
    #[arg(long)]
    pub symbol: String,
    /// Expiry date, 8-digit YYYYMMDD
    #[arg(long)]
    pub expiry: String,
    /// Strike price (finite, > 0)
    #[arg(long)]
    pub strike: f64,
    /// Right: C|CALL or P|PUT (case-insensitive)
    #[arg(long)]
    pub right: String,
    /// Quantity in whole contracts (>= 1)
    #[arg(long)]
    pub qty: f64,
    /// Limit price (REQUIRED — v1 is LMT-only, no MKT; finite, > 0)
    #[arg(long)]
    pub limit: f64,
    /// Exchange
    #[arg(long, default_value = "SMART")]
    pub exchange: String,
    /// Currency
    #[arg(long, default_value = "USD")]
    pub currency: String,
    /// Trading class, e.g. SPXW (optional; gateway resolves when absent)
    #[arg(long)]
    pub trading_class: Option<String>,
}

/// omi option-combo --action BUY --qty 1 --limit 0.05 --leg "BUY 1 AAPL 20260918 240 C" --leg "SELL 1 AAPL 20260918 250 C"
#[derive(Args, Debug)]
pub struct OptionComboArgs {
    /// Combo side: BUY or SELL (the overall order action)
    #[arg(long)]
    pub action: String,
    /// Quantity in whole contracts (>= 1)
    #[arg(long)]
    pub qty: f64,
    /// Net limit price (REQUIRED — LMT-only; negative = credit; allow_hyphen_values for -0.50)
    #[arg(long, allow_hyphen_values = true)]
    pub limit: f64,
    /// Leg specs: "ACTION RATIO SYMBOL EXPIRY STRIKE RIGHT" (2..=4 legs required)
    #[arg(long = "leg", required = true)]
    pub legs: Vec<String>,
    /// Exchange
    #[arg(long, default_value = "SMART")]
    pub exchange: String,
    /// Currency
    #[arg(long, default_value = "USD")]
    pub currency: String,
}

/// omi option-close --conid 123456789 --limit 3.20 [--qty 1]
#[derive(Args, Debug)]
pub struct OptionCloseArgs {
    /// Conid of the HELD option position to close (from `omi positions`)
    #[arg(long)]
    pub conid: i32,
    /// Limit price (REQUIRED — LMT-only, no MKT; finite, > 0)
    #[arg(long)]
    pub limit: f64,
    /// Quantity to close in whole contracts (>= 1); omit to close the full position
    #[arg(long)]
    pub qty: Option<f64>,
}
