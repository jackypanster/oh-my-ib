//! `grid` driver — one paper-only reconcile tick (ADR 0033). Gateway-dependent, reviewed-by-
//! reading + operator acceptance (NOT frozen). Single connection: one `account_updates` drain
//! (cash+positions) + one `all_open_orders` drain, then execute the planner's actions in order.
//!
//! Containment (ADR 0017): this module contains NO raw `place_order`/`cancel_order`. It composes
//! the existing `trade.rs` choke points — `build_stk_order` (pure build), `place_with_client`
//! (placement body, already account-stamped per ADR 0024), `cancel_with_client` (bounded cancel
//! ack) — so write calls stay confined to `trade.rs`. Paper-only in v1: a live port is refused
//! offline, before connect (the grid places multiple orders per tick — keep it off real money
//! until it has its own live ADR).
//!
//! Execution: actions are already ordered all-Cancels-before-all-Places (the planner's invariant).
//! Drain them in order; STOP on the first error (record + return partial — no blind retry: the
//! next tick re-reconciles from the live state). A `--dry-run` returns the planned actions only.

use ibapi::orders::Action;
use serde_json::{json, Value};

use crate::cli::GridTickArgs;
use crate::config::{Config, LIVE_PORT};
use crate::error::AppError;
use crate::grid::{self, Action as GridAction, OpenOrderLite, Side};

/// Run one grid reconcile tick. Paper-only; live ports are refused offline.
pub fn grid_tick(cfg: &Config, args: &GridTickArgs) -> Result<Value, AppError> {
    let gcfg = grid::GridConfig::load(&args.config)?;
    if cfg.port == LIVE_PORT {
        return Err(AppError::config(
            "grid-tick is paper-only in v1 — use paper :4002",
            "grid-tick",
        ));
    }
    let client = super::connect(cfg)?;
    let account = super::resolve_account(&client, cfg)?;
    let (snap, positions) = super::account::read_account_positions(&client, &account)?;
    let open_val =
        super::orders::open_orders_with_client(&client, Some(account.0.as_str()), "grid-tick")?;
    let open = map_open_orders(&open_val);
    let actions = grid::plan_grid_tick(&gcfg, &snap, &positions, &open);

    let planned = actions.len();
    if args.dry_run {
        return Ok(json!({
            "account": account.0,
            "dry_run": true,
            "planned": planned,
            "actions": actions.iter().map(action_echo).collect::<Vec<_>>(),
        }));
    }

    let mut results: Vec<Value> = Vec::new();
    for a in &actions {
        let outcome = execute(a, &client, &account);
        match outcome {
            Ok(v) => results.push(v),
            Err(e) => {
                // Stop on first error: record the partial results + the stopping error, then
                // return Ok (the tick ran; the envelope reports what happened). No retry.
                return Ok(json!({
                    "account": account.0,
                    "dry_run": false,
                    "planned": planned,
                    "executed": results.len(),
                    "actions": results,
                    "stopped_at": action_echo(a),
                    "error": { "code": e.code(), "message": e.message },
                }));
            }
        }
    }

    Ok(json!({
        "account": account.0,
        "dry_run": false,
        "planned": planned,
        "executed": results.len(),
        "actions": results,
    }))
}

/// Execute one planned action on the shared client.
fn execute(
    a: &GridAction,
    client: &ibapi::client::blocking::Client,
    account: &ibapi::accounts::types::AccountId,
) -> Result<Value, AppError> {
    match a {
        GridAction::Cancel { order_id } => super::trade::cancel_with_client(client, *order_id),
        GridAction::Place { symbol, side, qty, limit } => {
            let ib_action = match side {
                Side::Buy => Action::Buy,
                Side::Sell => Action::Sell,
            };
            let (contract, order) =
                super::trade::build_stk_order(symbol, ib_action, *qty, Some(*limit));
            super::trade::place_with_client(
                client,
                "grid-tick",
                &contract,
                &order,
                account,
                |id, status| {
                    json!({
                        "order_id": id,
                        "status": status,
                        "symbol": symbol,
                        "action": side_str(*side),
                        "quantity": qty,
                        "limit_price": limit,
                    })
                },
            )
        }
    }
}

/// The JSON echo of a planned action (used by --dry-run and the `stopped_at` field).
fn action_echo(a: &GridAction) -> Value {
    match a {
        GridAction::Cancel { order_id } => json!({ "cancel": order_id }),
        GridAction::Place { symbol, side, qty, limit } => json!({
            "place": {
                "symbol": symbol,
                "action": side_str(*side),
                "quantity": qty,
                "limit_price": limit,
            }
        }),
    }
}

fn side_str(s: Side) -> &'static str {
    match s {
        Side::Buy => "Buy",
        Side::Sell => "Sell",
    }
}

/// Map the `open_orders_with_client` JSON rows into the trimmed `OpenOrderLite` view the planner
/// consumes. Orders whose action isn't Buy/Sell (e.g. an option leg) are skipped — the grid is
/// STK-only and would only mis-match them. A null `limit_price` (a resting MKT order) becomes
/// 0.0, which never matches a desired LMT rung, so such an order is cancelled if on a
/// configured symbol (correct: a stale MKT has no place in a LMT grid).
fn map_open_orders(val: &Value) -> Vec<OpenOrderLite> {
    let mut out = Vec::new();
    let Some(arr) = val.as_array() else {
        return out;
    };
    for row in arr {
        let Some(side) = row["action"].as_str().and_then(side_from_str) else {
            continue;
        };
        out.push(OpenOrderLite {
            order_id: row["order_id"].as_i64().unwrap_or(0) as i32,
            symbol: row["symbol"].as_str().unwrap_or("").to_string(),
            side,
            limit: row["limit_price"].as_f64().unwrap_or(0.0),
            qty: row["quantity"].as_f64().unwrap_or(0.0),
        });
    }
    out
}

fn side_from_str(s: &str) -> Option<Side> {
    match s {
        "Buy" => Some(Side::Buy),
        "Sell" => Some(Side::Sell),
        _ => None,
    }
}
