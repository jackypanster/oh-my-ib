//! `trade` — the ONLY module with write calls (Phase 2 write path, ADR 0017).
//!
//! Safety architecture (ADR 0017):
//! - **Containment**: `place_order`/`cancel_order` live HERE ONLY. Review greps for these
//!   symbols elsewhere — zero hits expected. No read command imports this module's gateway fns.
//! - **Double gate on live, ungated paper**: an order at the effective live port
//!   (`cfg.port == LIVE_PORT`, catching both `--live` and hand-set `--port 4001`) requires
//!   `OMI_ALLOW_LIVE=1`; missing ⇒ `code="config"` BEFORE any connection (offline-deterministic).
//! - **Bounded, deterministic ack**: allocate the order id FIRST (`next_valid_order_id`), then
//!   `place_order` and wait for the FIRST `OrderStatus`/`OpenOrder` event under
//!   `TAKE_FIRST_TIMEOUT` (per-item window, ADR 0016's bounded-iterator pattern). No event
//!   ⇒ exit 6 `timeout` envelope that NAMES the allocated order id, says it MAY be submitted,
//!   points at `omi orders`, and forbids blind retry.
//! - **No retry, ever**: a placement timeout is an UNKNOWN state, not a failure to redo —
//!   automatic re-placement is the classic double-order bug.

use ibapi::contracts::Contract;
use ibapi::orders::{Action, CancelOrder, Order, PlaceOrder, TimeInForce};
use serde_json::{json, Value};

use crate::cli::{CancelArgs, OrderArgs};
use crate::config::{Config, LIVE_PORT};
use crate::error::AppError;

/// Pure, FROZEN seam: CLI params → the exact `(Contract, Order)` pair sent to the gateway.
/// `limit` `None` ⇒ MKT, `Some` ⇒ LMT. TIF always `Day` (v1). `Contract::stock` uses the
/// SMART/USD defaults (parity with `quote`/`contract`).
pub fn build_stk_order(symbol: &str, side: Action, quantity: f64, limit: Option<f64>) -> (Contract, Order) {
    let contract = Contract::stock(symbol).build();
    let mut order = Order {
        action: side,
        total_quantity: quantity,
        tif: TimeInForce::Day,
        ..Default::default()
    };
    match limit {
        Some(px) => {
            order.order_type = "LMT".into();
            order.limit_price = Some(px);
        }
        None => {
            order.order_type = "MKT".into();
        }
    }
    (contract, order)
}

/// Pure, FROZEN seam: the ack JSON — exact 6-key object. `order_id`/`status` come from
/// allocation + the first ack event; `symbol`/`action`/`quantity`/`limit_price` echo the
/// request. MKT ⇒ `limit_price: null` (key present, value null).
pub fn shape_order_ack(
    order_id: i32,
    status: &str,
    symbol: &str,
    action: &str,
    quantity: f64,
    limit_price: Option<f64>,
) -> Value {
    json!({
        "order_id": order_id,
        "status": status,
        "symbol": symbol,
        "action": action,
        "quantity": quantity,
        "limit_price": limit_price,
    })
}

/// The double gate (ADR 0017). MUST run before `super::connect` — offline-deterministic.
/// Gates on the EFFECTIVE live port (covers both `--live` and a hand-set `--port 4001`).
/// Paper (`:4002`, the default) is ungated.
pub fn require_live_write_gate(cfg: &Config) -> Result<(), AppError> {
    if cfg.port == LIVE_PORT && std::env::var("OMI_ALLOW_LIVE").as_deref() != Ok("1") {
        return Err(AppError::config(
            "live order rejected: set OMI_ALLOW_LIVE=1 to enable live trading (paper :4002 needs no gate)",
            "live write gate",
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Gateway fns (review-by-reading; NOT frozen — needs a live gateway)
// ---------------------------------------------------------------------------

/// Place a BUY order. See [`place`] for the shared placement logic.
pub fn buy(cfg: &Config, args: &OrderArgs) -> Result<Value, AppError> {
    place(cfg, args, Action::Buy, "buy")
}

/// Place a SELL order. See [`place`] for the shared placement logic.
pub fn sell(cfg: &Config, args: &OrderArgs) -> Result<Value, AppError> {
    place(cfg, args, Action::Sell, "sell")
}

/// Cancel an order by id. Gate → connect → `cancel_order` → bounded first ack.
pub fn cancel(cfg: &Config, args: &CancelArgs) -> Result<Value, AppError> {
    require_live_write_gate(cfg)?;
    let client = super::connect(cfg)?;
    let subscription = client
        .cancel_order(args.order_id, "")
        .map_err(|e| AppError::data(format!("cancel_order failed: {e}"), "cancel"))?;
    // Bounded first-ack: CancelOrder has only the OrderStatus variant (no events to skip),
    // so a single `.next()` under TAKE_FIRST_TIMEOUT suffices. Any None before the ack =
    // UNKNOWN state (the cancel MAY or MAY NOT have succeeded).
    match subscription.timeout_iter_data(super::TAKE_FIRST_TIMEOUT).next() {
        Some(Ok(CancelOrder::OrderStatus(os))) => Ok(json!({
            "order_id": args.order_id,
            "status": format!("{:?}", os.status),
        })),
        Some(Err(e)) => Err(AppError::data(
            format!("cancel order stream: {e}"),
            "cancel",
        )),
        None => Err(AppError::timeout(
            format!(
                "cancel of order {} — no ack within {}s; the cancel MAY or MAY NOT have \
                 succeeded — verify with `omi orders`, do NOT retry blindly",
                args.order_id,
                super::TAKE_FIRST_TIMEOUT.as_secs()
            ),
            "cancel",
        )),
    }
}

/// Shared placement core for buy/sell. Ordering invariant (frozen-test-dependent):
/// local validation → gate → connect → allocate id → build → place → bounded first-ack.
fn place(cfg: &Config, args: &OrderArgs, side: Action, ctx: &str) -> Result<Value, AppError> {
    // 1. Local validation (usage errors, before gate and connect).
    if args.quantity <= 0.0 {
        return Err(AppError::usage(
            format!("quantity must be positive, got {}", args.quantity),
            ctx,
        ));
    }
    if let Some(px) = args.limit {
        if px <= 0.0 {
            return Err(AppError::usage(
                format!("limit price must be positive, got {px}"),
                ctx,
            ));
        }
    }

    // 2. Double gate (config error, before connect — offline-deterministic).
    require_live_write_gate(cfg)?;

    // 3. Connect (connection errors).
    let client = super::connect(cfg)?;

    // 4. Allocate the order id FIRST so even a timeout error can NAME it.
    let order_id = client
        .next_valid_order_id()
        .map_err(|e| AppError::data(format!("next_valid_order_id failed: {e}"), ctx))?;

    // 5. Build + place.
    let (contract, order) = build_stk_order(&args.symbol, side, args.quantity, args.limit);
    let subscription = client
        .place_order(order_id, &contract, &order)
        .map_err(|e| AppError::data(format!("place_order failed: {e}"), ctx))?;

    // 6. Bounded first-ack loop (ADR 0016 bounded-iterator pattern). Take the FIRST
    //    OrderStatus or OpenOrder event; skip ExecutionData/CommissionReport (window
    //    refreshes on each arrival). Any None before an ack = UNKNOWN state (the order
    //    MAY have been submitted) — never a silent success, never a blind retry.
    let side_str = format!("{:?}", side);
    let mut items = subscription.timeout_iter_data(super::TAKE_FIRST_TIMEOUT);
    loop {
        match items.next() {
            Some(Ok(PlaceOrder::OrderStatus(os))) => {
                return Ok(shape_order_ack(
                    order_id,
                    &format!("{:?}", os.status),
                    &args.symbol,
                    &side_str,
                    args.quantity,
                    args.limit,
                ));
            }
            Some(Ok(PlaceOrder::OpenOrder(od))) => {
                return Ok(shape_order_ack(
                    order_id,
                    &format!("{:?}", od.order_state.status),
                    &args.symbol,
                    &side_str,
                    args.quantity,
                    args.limit,
                ));
            }
            Some(Ok(_)) => {} // ExecutionData / CommissionReport — skip, window refreshes.
            Some(Err(e)) => {
                return Err(AppError::data(
                    format!("order stream: {e}"),
                    ctx,
                ))
            }
            None => {
                return Err(AppError::timeout(
                    format!(
                        "order {} may have been SUBMITTED — no ack within {}s; verify with \
                         `omi orders`, do NOT retry blindly",
                        order_id,
                        super::TAKE_FIRST_TIMEOUT.as_secs()
                    ),
                    ctx,
                ))
            }
        }
    }
}
