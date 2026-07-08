//! The "surface of record": the authoritative agent-facing description of every
//! `omi` subcommand.
//!
//! - [`command_name`] is the shared anchor: an EXHAUSTIVE match over [`Command`]
//!   with no `_` arm, so adding a `Command` variant is a compile error until it
//!   is named here. The audit `cmd` field (ADR 0036, card 02) feeds off the same
//!   name, so help + audit cannot drift.
//! - The [`REGISTRY`] carries what clap metadata cannot: a usage example and the
//!   write-gate marker per AGENTS.md hard safety rules.
//! - [`help_json`] is the one-shot `omi help` payload: the entire surface in one
//!   invocation, agent-parseable, no gateway/config needed.

use serde_json::{json, Value};

use crate::cli::Command;

/// Write-gate markers mirroring AGENTS.md hard safety rules.
const READ_ONLY: &str = "read-only";
const WRITE: &str = "write";
const WRITE_PAPER_ONLY: &str = "write-paper-only";

/// The stable kebab-case name of a command.
///
/// EXHAUSTIVE — no `_` arm: a new `Command` variant fails to build until it is
/// named here. The audit seam (card 02) calls this for the `cmd` field, so help
/// and audit share one anchor and cannot diverge.
pub fn command_name(cmd: &Command) -> &'static str {
    match cmd {
        Command::Health => "health",
        Command::Brief => "brief",
        Command::Account => "account",
        Command::Pnl => "pnl",
        Command::PnlByPosition => "pnl-by-position",
        Command::Positions => "positions",
        Command::Orders => "orders",
        Command::Executions => "executions",
        Command::Quote(_) => "quote",
        Command::Contract(_) => "contract",
        Command::History(_) => "history",
        Command::Search(_) => "search",
        Command::CompletedOrders => "completed-orders",
        Command::Buy(_) => "buy",
        Command::Sell(_) => "sell",
        Command::Cancel(_) => "cancel",
        Command::OptionChain(_) => "option-chain",
        Command::OptionQuote(_) => "option-quote",
        Command::OptionBuy(_) => "option-buy",
        Command::OptionSell(_) => "option-sell",
        Command::OptionCombo(_) => "option-combo",
        Command::OptionClose(_) => "option-close",
        Command::SmaSignal(_) => "sma-signal",
        Command::GridTick(_) => "grid-tick",
        Command::SmaTick(_) => "sma-tick",
        Command::Help => "help",
        Command::Logs(_) => "logs",
    }
}

struct Entry {
    name: &'static str,
    purpose: &'static str,
    usage: &'static str,
    example: &'static str,
    gate: &'static str,
}

/// The full command surface. Order follows the `Command` enum declaration so a
/// reader can diff the two side by side. Every field is non-empty — the frozen
/// help spec asserts this.
const REGISTRY: &[Entry] = &[
    Entry {
        name: "health",
        purpose: "Check the gateway connection and report managed accounts",
        usage: "omi health",
        example: "omi health",
        gate: READ_ONLY,
    },
    Entry {
        name: "brief",
        purpose: "Daily account snapshot: summary, PnL, positions, orders, executions (one connection)",
        usage: "omi brief",
        example: "omi brief",
        gate: READ_ONLY,
    },
    Entry {
        name: "account",
        purpose: "Account summary (net liq, cash, buying power)",
        usage: "omi account",
        example: "omi account",
        gate: READ_ONLY,
    },
    Entry {
        name: "pnl",
        purpose: "Account PnL (daily, unrealized, realized)",
        usage: "omi pnl",
        example: "omi pnl",
        gate: READ_ONLY,
    },
    Entry {
        name: "pnl-by-position",
        purpose: "Per-position PnL (daily, unrealized, realized)",
        usage: "omi pnl-by-position",
        example: "omi pnl-by-position",
        gate: READ_ONLY,
    },
    Entry {
        name: "positions",
        purpose: "Current positions",
        usage: "omi positions",
        example: "omi positions",
        gate: READ_ONLY,
    },
    Entry {
        name: "orders",
        purpose: "Open (working) orders — read only",
        usage: "omi orders",
        example: "omi orders",
        gate: READ_ONLY,
    },
    Entry {
        name: "executions",
        purpose: "Current-day executions (fills), joined to commission reports",
        usage: "omi executions",
        example: "omi executions",
        gate: READ_ONLY,
    },
    Entry {
        name: "quote",
        purpose: "Snapshot quote for one or more symbols",
        usage: "omi quote SYMBOL... [--sec-type STK] [--exchange SMART] [--currency USD]",
        example: "omi quote AAPL MSFT",
        gate: READ_ONLY,
    },
    Entry {
        name: "contract",
        purpose: "Resolve contract details for a symbol",
        usage: "omi contract SYMBOL [--sec-type STK]",
        example: "omi contract AAPL",
        gate: READ_ONLY,
    },
    Entry {
        name: "history",
        purpose: "Historical bars for a symbol",
        usage: "omi history SYMBOL [--bar 1d] [--duration 1M]",
        example: "omi history AAPL --bar 1d --duration 1M",
        gate: READ_ONLY,
    },
    Entry {
        name: "search",
        purpose: "Fuzzy symbol/company search",
        usage: "omi search PATTERN",
        example: "omi search apple",
        gate: READ_ONLY,
    },
    Entry {
        name: "completed-orders",
        purpose: "Today's completed orders (filled/cancelled) with status",
        usage: "omi completed-orders",
        example: "omi completed-orders",
        gate: READ_ONLY,
    },
    Entry {
        name: "buy",
        purpose: "Place a BUY order (paper by default; live needs --live + OMI_ALLOW_LIVE=1). Live must be LMT; notional capped at OMI_MAX_NOTIONAL",
        usage: "omi buy SYMBOL QTY [--limit P] [--outside-rth]",
        example: "omi buy QQQM 10 --limit 55.50",
        gate: WRITE,
    },
    Entry {
        name: "sell",
        purpose: "Place a SELL order (paper by default; live needs --live + OMI_ALLOW_LIVE=1). Live must be LMT; notional capped at OMI_MAX_NOTIONAL",
        usage: "omi sell SYMBOL QTY [--limit P] [--outside-rth]",
        example: "omi sell QQQM 10 --limit 55.50",
        gate: WRITE,
    },
    Entry {
        name: "cancel",
        purpose: "Cancel an order by id (paper by default; live needs --live + OMI_ALLOW_LIVE=1)",
        usage: "omi cancel ORDER_ID",
        example: "omi cancel 42",
        gate: WRITE,
    },
    Entry {
        name: "option-chain",
        purpose: "Option chain (expirations × strikes) for an underlying",
        usage: "omi option-chain SYMBOL [--exchange SMART]",
        example: "omi option-chain AAPL",
        gate: READ_ONLY,
    },
    Entry {
        name: "option-quote",
        purpose: "Snapshot quote + greeks for one option contract",
        usage: "omi option-quote --symbol SYM --expiry YYYYMMDD --strike K --right C|P [--exchange SMART] [--currency USD] [--trading-class SPXW]",
        example: "omi option-quote --symbol AAPL --expiry 20260918 --strike 250 --right C",
        gate: READ_ONLY,
    },
    Entry {
        name: "option-buy",
        purpose: "Place a single-leg option BUY (LMT/DAY; paper default; live needs --live + OMI_ALLOW_LIVE=1)",
        usage: "omi option-buy --symbol SYM --expiry YYYYMMDD --strike K --right C|P --qty N --limit P [--exchange SMART] [--currency USD] [--trading-class SPXW]",
        example: "omi option-buy --symbol AAPL --expiry 20260918 --strike 250 --right C --qty 1 --limit 5.50",
        gate: WRITE,
    },
    Entry {
        name: "option-sell",
        purpose: "Place a single-leg option SELL (LMT/DAY; paper default; live needs --live + OMI_ALLOW_LIVE=1)",
        usage: "omi option-sell --symbol SYM --expiry YYYYMMDD --strike K --right C|P --qty N --limit P [--exchange SMART] [--currency USD] [--trading-class SPXW]",
        example: "omi option-sell --symbol AAPL --expiry 20260918 --strike 250 --right C --qty 1 --limit 5.50",
        gate: WRITE,
    },
    Entry {
        name: "option-combo",
        purpose: "Place a multi-leg option combo (BAG, LMT/DAY; paper default; live needs --live + OMI_ALLOW_LIVE=1)",
        usage: "omi option-combo --action BUY|SELL --qty N --limit P --leg \"ACTION RATIO SYMBOL EXPIRY STRIKE RIGHT\"... [--exchange SMART] [--currency USD]",
        example: "omi option-combo --action BUY --qty 1 --limit 0.05 --leg \"BUY 1 AAPL 20260918 240 C\" --leg \"SELL 1 AAPL 20260918 250 C\"",
        gate: WRITE,
    },
    Entry {
        name: "option-close",
        purpose: "Close a HELD option position by conid (side derived from held position; LMT/DAY; paper default; live needs --live + OMI_ALLOW_LIVE=1). Refuses while a working close order exists on the conid",
        usage: "omi option-close --conid CONID --limit P [--qty N]",
        example: "omi option-close --conid 123456789 --limit 3.20",
        gate: WRITE,
    },
    Entry {
        name: "sma-signal",
        purpose: "200-day SMA month-end HOLD/EXIT timing signal (read-only). No args = current positions",
        usage: "omi sma-signal [SYMBOLS...] [--sma N]",
        example: "omi sma-signal QQQM --sma 200",
        gate: READ_ONLY,
    },
    Entry {
        name: "grid-tick",
        purpose: "Run one grid reconcile tick (paper-only — refuses the live port). Reads the grid config, snapshots account + positions + open orders, plans buy/sell rungs, and (unless --dry-run) executes",
        usage: "omi grid-tick --config grid.toml [--dry-run]",
        example: "omi grid-tick --config grid.toml --dry-run",
        gate: WRITE_PAPER_ONLY,
    },
    Entry {
        name: "sma-tick",
        purpose: "Reconcile the position to the 200-day month-end signal (paper-only — refuses the live port). HOLD ⇒ buy up to lot, EXIT ⇒ sell to flat, INSUFFICIENT ⇒ no trade",
        usage: "omi sma-tick [SYMBOL] [--lot N] [--sma N] [--dry-run]",
        example: "omi sma-tick QQQM --lot 10 --dry-run",
        gate: WRITE_PAPER_ONLY,
    },
    Entry {
        name: "help",
        purpose: "One-shot JSON command surface: the entire command registry (name/purpose/usage/example/gate) in a single invocation. Agent-parseable; no gateway, no config needed",
        usage: "omi help",
        example: "omi help",
        gate: READ_ONLY,
    },
    Entry {
        name: "logs",
        purpose: "Read the invocation audit log tail (JSONL). Local only — no gateway, no config",
        usage: "omi logs [--tail N]",
        example: "omi logs --tail 50",
        gate: READ_ONLY,
    },
];

/// The one-shot `omi help` JSON payload: the entire command surface plus the
/// global flags. Shape (additive-only evolution):
///
/// ```json
/// {"global": {"flags": [{"name": "--format", "doc": "..."}, ...]},
///  "commands": [{"name": "buy", "purpose": "...", "usage": "...",
///                "example": "...", "gate": "write"}, ...]}
/// ```
pub fn help_json() -> Value {
    json!({
        "global": {
            "flags": [
                {"name": "--format", "doc": "Output format: json (default) | table"},
                {"name": "--host", "doc": "Gateway host (default: 127.0.0.1)"},
                {"name": "--port", "doc": "Gateway port (default: 4002 paper; 4001 live)"},
                {"name": "--live", "doc": "Use the LIVE account (port 4001) instead of paper"},
                {"name": "--preview", "doc": "Preview the order via IB whatIf (no transmit): resolved contract + margin/commission, no order placed"},
                {"name": "--client-id", "doc": "API client id (default: 100)"},
                {"name": "--account", "doc": "Account id (default: first managed account)"},
                {"name": "--md-type", "doc": "Market data type: live|delayed|frozen (default: config / delayed)"},
            ]
        },
        "commands": REGISTRY.iter().map(|e| json!({
            "name": e.name,
            "purpose": e.purpose,
            "usage": e.usage,
            "example": e.example,
            "gate": e.gate,
        })).collect::<Vec<_>>(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;
    use std::collections::BTreeSet;

    /// Staleness guard: clap's subcommand name set must equal the registry name
    /// set. A `Command` variant that parses but is absent from the registry (or a
    /// dangling registry entry) fails here in either direction. The frozen help
    /// spec pins today's 27; this guard is the future-proof backstop.
    #[test]
    fn registry_names_match_clap_subcommands() {
        let clap_names: BTreeSet<String> = <crate::cli::Cli as CommandFactory>::command()
            .get_subcommands()
            .map(|c| c.get_name().to_string())
            .collect();
        let registry_names: BTreeSet<String> =
            REGISTRY.iter().map(|e| e.name.to_string()).collect();
        assert_eq!(
            clap_names, registry_names,
            "help registry drifted from the CLI surface"
        );
    }

    /// `command_name` is exhaustive over `Command` — the registry must cover the
    /// same variants, so every registry name resolves back through it.
    #[test]
    fn every_registry_name_is_a_command_name() {
        // Build one of each variant is impractical (many take args); instead
        // assert the static invariant: REGISTRY names are a subset of the names
        // `command_name` can return. The clap-vs-registry test above already
        // proves coverage of the variant set; this just guards against typos in
        // the registry `name` strings themselves (e.g. a stray "helth").
        let known: BTreeSet<&str> = [
            "health", "brief", "account", "pnl", "pnl-by-position", "positions",
            "orders", "executions", "quote", "contract", "history", "search",
            "completed-orders", "buy", "sell", "cancel", "option-chain",
            "option-quote", "option-buy", "option-sell", "option-combo",
            "option-close", "sma-signal", "grid-tick", "sma-tick", "help", "logs",
        ]
        .into_iter()
        .collect();
        for e in REGISTRY {
            assert!(known.contains(e.name), "registry name `{}` is not a known command", e.name);
        }
    }
}
