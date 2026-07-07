//! `sma_tick` — the active 200-day month-end timing executor (ADR 0035). Paper-only WRITE driver.
//!
//! Mirrors the grid-tick shape: a PURE reconcile planner (FROZEN) + a thin gateway driver
//! (review-by-reading). The driver composes the existing `trade.rs` choke points — NO raw
//! `place_order`/`cancel_order` here (ADR 0017 containment holds).
//!
//! Binary target: HOLD ⇒ `lot` shares, EXIT ⇒ 0, INSUFFICIENT ⇒ don't trade. The planner computes
//! the delta from the current qty and returns `Buy(delta)` / `Sell(-delta)` / `Noop`. The gateway
//! reuses `signal_for` (the single-symbol signal computation shared with `sma_signal_cmd`), reads
//! the current position, plans, and (unless `--dry-run`) places ONE marketable LMT via
//! `build_stk_order` + `place_with_client` (Buy @ latest_close×1.02, Sell @ latest_close×0.98).

use ibapi::orders::Action;
use serde_json::{json, Value};

use crate::cli::SmaTickArgs;
use crate::config::{Config, LIVE_PORT};
use crate::error::AppError;
use crate::ib::SignalState;

/// A planned reconcile action. `qty` is always the POSITIVE magnitude of the trade.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TickAction {
    Buy { qty: f64 },
    Sell { qty: f64 },
    Noop,
}

/// Pure reconcile planner (ADR 0035, FROZEN). Binary target: HOLD ⇒ `lot`, EXIT ⇒ 0,
/// INSUFFICIENT ⇒ don't trade. `delta = target - current_qty` → Buy(delta) / Sell(-delta) / Noop.
/// Uses `>`/`<` against a 1e-9 epsilon (no `==` on f64).
pub fn plan_sma_tick(state: SignalState, current_qty: f64, lot: f64) -> TickAction {
    let target = match state {
        SignalState::Hold => lot,
        SignalState::Exit => 0.0,
        SignalState::Insufficient => return TickAction::Noop,
    };
    let delta = target - current_qty;
    if delta > 1e-9 {
        TickAction::Buy { qty: delta }
    } else if delta < -1e-9 {
        TickAction::Sell { qty: -delta }
    } else {
        TickAction::Noop
    }
}

/// Round to 2 decimal places (cents) — the marketable-LMT price tick.
fn round2(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

/// The machine-readable shape of a planned action (used in the JSON envelope).
fn action_str(a: TickAction) -> &'static str {
    match a {
        TickAction::Buy { .. } => "BUY",
        TickAction::Sell { .. } => "SELL",
        TickAction::Noop => "NOOP",
    }
}

/// The gateway driver: paper-only guard → connect → signal → position → plan → (dry-run | place).
/// Paper-only in v1: a live port is refused offline, before connect.
pub fn sma_tick_cmd(cfg: &Config, args: &SmaTickArgs) -> Result<Value, AppError> {
    if cfg.port == LIVE_PORT {
        return Err(AppError::config(
            "sma-tick is paper-only in v1 — use paper :4002",
            "sma-tick",
        ));
    }
    // Validate the SMA window BEFORE connect (mirrors sma_signal_cmd's guard).
    if args.sma < 1 {
        return Err(AppError::config(
            format!("--sma must be >= 1, got {}", args.sma),
            "sma-tick",
        ));
    }
    // Validate the target lot BEFORE connect: a non-positive or non-finite lot makes the binary
    // target nonsensical (e.g. --lot=-10 flips HOLD into a short; --lot=inf reaches the builder).
    // build_stk_order is a pure builder with no validation, so the gateway must guard here.
    if !args.lot.is_finite() || args.lot <= 0.0 {
        return Err(AppError::config(
            format!("--lot must be a positive number, got {}", args.lot),
            "sma-tick",
        ));
    }
    let symbol = args.symbol.clone().unwrap_or_else(|| "QQQM".into());
    let client = super::connect(cfg)?;
    let account = super::resolve_account(&client, cfg)?;
    let sig = super::signal::signal_for(&client, &symbol, args.sma)?;
    let current_qty = current_position_qty(cfg, &symbol)?;
    let action = plan_sma_tick(sig.state, current_qty, args.lot);
    let target_qty = match sig.state {
        SignalState::Hold => args.lot,
        SignalState::Exit => 0.0,
        SignalState::Insufficient => current_qty,
    };
    let as_of = format!(
        "{:04}-{:02}",
        sig.as_of_month_end.0, sig.as_of_month_end.1
    );

    if args.dry_run {
        return Ok(json!({
            "symbol": symbol,
            "signal": sig.state.as_str(),
            "as_of": as_of,
            "current_qty": current_qty,
            "target_qty": target_qty,
            "action": action_str(action),
            "dry_run": true,
        }));
    }

    let (side, qty, price) = match action {
        TickAction::Noop => {
            return Ok(json!({
                "symbol": symbol,
                "signal": sig.state.as_str(),
                "as_of": as_of,
                "current_qty": current_qty,
                "target_qty": target_qty,
                "action": "NOOP",
            }));
        }
        TickAction::Buy { qty } => (Action::Buy, qty, round2(sig.latest_close * 1.02)),
        TickAction::Sell { qty } => (Action::Sell, qty, round2(sig.latest_close * 0.98)),
    };

    let (contract, order) = super::trade::build_stk_order(&symbol, side, qty, Some(price));
    let order_result = super::trade::place_with_client(
        &client,
        "sma-tick",
        &contract,
        &order,
        &account,
        |id, status| {
            json!({
                "order_id": id,
                "status": status,
                "symbol": symbol,
                "action": format!("{:?}", side),
                "quantity": qty,
                "limit_price": price,
            })
        },
    )?;

    Ok(json!({
        "symbol": symbol,
        "signal": sig.state.as_str(),
        "as_of": as_of,
        "current_qty": current_qty,
        "target_qty": target_qty,
        "action": action_str(action),
        "order": order_result,
    }))
}

/// Read the current held qty for `symbol` from the read-only `positions` drain. Fail-closed: a read
/// ERROR propagates (a write command must not fabricate a flat position from unknown state — HOLD
/// would double-buy, EXIT would skip the close). Only "symbol absent from a SUCCESSFUL payload" is
/// treated as 0.0. Reuses the `positions(cfg)` path (a second short-lived connection, acceptable for
/// a single-tick paper command).
fn current_position_qty(cfg: &Config, symbol: &str) -> Result<f64, AppError> {
    let val = super::positions(cfg)?;
    let qty = val["positions"]
        .as_array()
        .and_then(|rows| {
            rows.iter().find_map(|r| {
                if r["symbol"].as_str() == Some(symbol) {
                    r["qty"].as_f64()
                } else {
                    None
                }
            })
        })
        .unwrap_or(0.0);
    Ok(qty)
}
