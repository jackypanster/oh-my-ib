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

use ibapi::contracts::{Contract, OptionRight};
use ibapi::orders::{Action, CancelOrder, Order, PlaceOrder, TimeInForce};
use serde_json::{json, Value};

use crate::cli::{CancelArgs, OptionOrderArgs, OrderArgs};
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

/// Pure, FROZEN seam: validated option params → the exact `(Contract, Order)`. LMT-only
/// (ADR 0020 D2): `order_type` always `"LMT"`, `limit_price` always `Some`, TIF always
/// `Day`. Contract via the options-read builder chain (SMART/USD/multiplier-100 defaults;
/// `trading_class` when given).
#[allow(clippy::too_many_arguments)] // the signature IS the frozen contract (brief.rs:27 precedent)
pub fn build_option_order(
    symbol: &str,
    expiry: (u16, u8, u8),
    strike: f64,
    right: OptionRight,
    trading_class: Option<&str>,
    exchange: &str,
    currency: &str,
    side: Action,
    quantity: f64,
    limit: f64,
) -> (Contract, Order) {
    let mut builder = match right {
        OptionRight::Call => Contract::call(symbol),
        _ => Contract::put(symbol), // OptionRight is non_exhaustive; only Call/Put exist today
    }
    .strike(strike)
    .expires_on(expiry.0, expiry.1, expiry.2)
    .on_exchange(exchange)
    .in_currency(currency);
    if let Some(tc) = trading_class {
        builder = builder.trading_class(tc);
    }
    let contract = builder.build();
    let order = Order {
        action: side,
        total_quantity: quantity,
        order_type: "LMT".into(),
        limit_price: Some(limit),
        tif: TimeInForce::Day,
        ..Default::default()
    };
    (contract, order)
}

/// Pure, FROZEN seam: the 9-key option ack (ADR 0020 D5). Echoes the request (expiry as the
/// original YYYYMMDD string; right normalized `"C"`|`"P"`; action as `"BUY"`/`"SELL"`) +
/// `order_id`/`status` from allocation + first ack. `limit_price` always a number (LMT-only).
#[allow(clippy::too_many_arguments)] // the 9-key ack shape IS the frozen contract (brief.rs:27 precedent)
pub fn shape_option_order_ack(
    order_id: i32,
    status: &str,
    symbol: &str,
    expiry: &str,
    strike: f64,
    right: &str,
    action: &str,
    quantity: f64,
    limit_price: f64,
) -> Value {
    json!({
        "order_id": order_id,
        "status": status,
        "symbol": symbol,
        "expiry": expiry,
        "strike": strike,
        "right": right,
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

/// Place a single-leg option BUY (LMT/DAY; paper default; live needs --live + OMI_ALLOW_LIVE=1).
pub fn option_buy(cfg: &Config, args: &OptionOrderArgs) -> Result<Value, AppError> {
    place_option(cfg, args, Action::Buy, "option-buy")
}

/// Place a single-leg option SELL (LMT/DAY; paper default; live needs --live + OMI_ALLOW_LIVE=1).
pub fn option_sell(cfg: &Config, args: &OptionOrderArgs) -> Result<Value, AppError> {
    place_option(cfg, args, Action::Sell, "option-sell")
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

/// Shared placement core (stk + option, ADR 0020 D7). Extracts the contract-agnostic
/// gate → connect → allocate → place → bounded first-ack steps. Behavior byte-identical
/// to the pre-refactor stk path — the frozen stk suite asserts it. `ack` maps
/// (order_id, status) → the verb-specific ack JSON, so the two ack shapes (6-key stk,
/// 9-key option) stay pure and disjoint.
fn place_core(
    cfg: &Config,
    ctx: &str,
    contract: &Contract,
    order: &Order,
    ack: impl Fn(i32, &str) -> Value,
) -> Result<Value, AppError> {
    // 1. Double gate (config error, before connect — offline-deterministic).
    require_live_write_gate(cfg)?;

    // 2. Connect (connection errors).
    let client = super::connect(cfg)?;

    // 3. Allocate the order id FIRST so even a timeout error can NAME it. Uses the
    //    handshake-seeded local allocator (client.next_order_id, ADR 0018): non-blocking,
    //    returns the id_manager's next id. The prior next_valid_order_id() was an unbounded
    //    subscription.next() that this gateway never answers (paper wedge, ADR 0018).
    let order_id = client.next_order_id();

    // 4. Place.
    let subscription = client
        .place_order(order_id, contract, order)
        .map_err(|e| AppError::data(format!("place_order failed: {e}"), ctx))?;

    // 5. Bounded first-ack loop (ADR 0016 bounded-iterator pattern). Take the FIRST
    //    OrderStatus or OpenOrder event; skip ExecutionData/CommissionReport (window
    //    refreshes on each arrival). Any None before an ack = UNKNOWN state (the order
    //    MAY have been submitted) — never a silent success, never a blind retry.
    let mut items = subscription.timeout_iter_data(super::TAKE_FIRST_TIMEOUT);
    loop {
        match items.next() {
            Some(Ok(PlaceOrder::OrderStatus(os))) => {
                return Ok(ack(order_id, &format!("{:?}", os.status)));
            }
            Some(Ok(PlaceOrder::OpenOrder(od))) => {
                return Ok(ack(order_id, &format!("{:?}", od.order_state.status)));
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

/// Stk placement: validation → build → place_core with the 6-key ack closure.
/// Ordering invariant (frozen-test-dependent): local validation → gate → connect.
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

    // 2. Build + place via the shared core (gate → connect → allocate → ack).
    let (contract, order) = build_stk_order(&args.symbol, side, args.quantity, args.limit);
    let side_str = format!("{:?}", side);
    place_core(cfg, ctx, &contract, &order, |id, status| {
        shape_order_ack(id, status, &args.symbol, &side_str, args.quantity, args.limit)
    })
}

/// Option placement: validation → build → place_core with the 9-key ack closure.
/// Ordering invariant (frozen): usage (local validation) < config (gate) < connection.
/// Option identity parsing shared with the read path via pub(crate) helpers (ADR 0020 D6).
fn place_option(
    cfg: &Config,
    args: &OptionOrderArgs,
    side: Action,
    ctx: &str,
) -> Result<Value, AppError> {
    // 1. Local validation (usage errors, before gate and connect). Ordering frozen:
    //    usage < config(gate) < connection.
    let right_str = super::option_quote::normalize_right(&args.right).ok_or_else(|| {
        AppError::usage(
            format!("invalid --right {}: expected C|CALL or P|PUT", args.right),
            ctx,
        )
    })?;
    let expiry = super::option_quote::parse_expiry(&args.expiry).ok_or_else(|| {
        AppError::usage(
            format!(
                "invalid --expiry {}: expected 8-digit YYYYMMDD with month 1-12 and day 1-31",
                args.expiry
            ),
            ctx,
        )
    })?;
    if !args.strike.is_finite() || args.strike <= 0.0 {
        return Err(AppError::usage(
            format!("--strike must be a finite positive number (got {})", args.strike),
            ctx,
        ));
    }
    if !args.qty.is_finite() || args.qty < 1.0 || args.qty.fract() != 0.0 {
        return Err(AppError::usage(
            format!("--qty must be a whole number of contracts >= 1 (got {})", args.qty),
            ctx,
        ));
    }
    if !args.limit.is_finite() || args.limit <= 0.0 {
        return Err(AppError::usage(
            format!("--limit must be a finite positive number (got {})", args.limit),
            ctx,
        ));
    }

    // 2. Build + place via the shared core (gate → connect → allocate → ack).
    let right = match right_str {
        "C" => OptionRight::Call,
        _ => OptionRight::Put,
    };
    let (contract, order) = build_option_order(
        &args.symbol,
        expiry,
        args.strike,
        right,
        args.trading_class.as_deref(),
        &args.exchange,
        &args.currency,
        side,
        args.qty,
        args.limit,
    );
    let action_str = format!("{:?}", side);
    place_core(cfg, ctx, &contract, &order, |id, status| {
        shape_option_order_ack(
            id,
            status,
            &args.symbol,
            &args.expiry,
            args.strike,
            right_str,
            &action_str,
            args.qty,
            args.limit,
        )
    })
}
