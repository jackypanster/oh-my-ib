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

use ibapi::accounts::types::AccountId;
use ibapi::accounts::{AccountPortfolioValue, AccountUpdate};
use ibapi::client::blocking::Client;
use ibapi::contracts::{Contract, LegAction, OptionRight, SecurityType};
use ibapi::orders::{Action, CancelOrder, Order, OrderState, PlaceOrder, TimeInForce};
use serde_json::{json, Value};

use crate::cli::{CancelArgs, OptionCloseArgs, OptionComboArgs, OptionOrderArgs, OrderArgs};
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

/// Pure, FROZEN seam (ADR 0026): the uniform whatIf preview envelope — exact 9-key object
/// echoed for ALL six order verbs. `Option<f64>` `None` ⇒ JSON `null` (key present, value
/// null): Tiger may honor `what_if` but leave margin/commission empty (CONTEXT.md R2); the
/// envelope stays a valid confirm card (echo + resolved contract). `state.warning_text`/
/// `state.status` map straight off `OrderState` (the mapping lives here, so it is frozen).
pub fn shape_preview(contract: &Contract, order: &Order, state: &OrderState) -> Value {
    json!({
        "preview": true,
        "what_if": true,
        "action": format!("{:?}", order.action),
        "contract": {
            "symbol": contract.symbol.to_string(),
            "sec_type": contract.security_type.to_string(),
            "conid": contract.contract_id,
        },
        "order": {
            "type": order.order_type,
            "qty": order.total_quantity,
            "limit": order.limit_price,
        },
        "margin": {
            "init_change": state.initial_margin_change,
            "maint_change": state.maintenance_margin_change,
            "equity_with_loan_change": state.equity_with_loan_change,
        },
        "commission": {
            "value": state.commission,
            "min": state.minimum_commission,
            "max": state.maximum_commission,
            "currency": state.commission_currency,
        },
        "warning": state.warning_text,
        "status": format!("{:?}", state.status),
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
// Option-close (close-by-conid) — pure seams (ADR 0022). Gateway fn below.
// ---------------------------------------------------------------------------

/// Pure, FROZEN seam (ADR 0022 §2): derive the close side + qty from a HELD option
/// position. The SIGN of `position` is the ONLY side authority — long (>0) ⇒ SELL, short
/// (<0) ⇒ BUY (the anti-double gate: a close never trusts user-declared direction).
/// `qty None` ⇒ full close (`|position|`); `Some(q)` ⇒ must be finite ∧ whole ∧ >= 1 ∧
/// <= `|position|` (over-close never flips a position). `position == 0` ⇒ Err.
pub fn derive_close(position: f64, qty: Option<f64>) -> Result<(Action, f64), String> {
    if position == 0.0 {
        return Err("position is 0 — nothing to close".to_string());
    }
    let side = if position > 0.0 { Action::Sell } else { Action::Buy };
    let abs = position.abs();
    let close_qty = match qty {
        None => abs,
        Some(q) => {
            if !q.is_finite() {
                return Err(format!("qty must be finite, got {q}"));
            }
            if q < 1.0 {
                return Err(format!("qty must be a whole number of contracts >= 1, got {q}"));
            }
            if q.fract() != 0.0 {
                return Err(format!("qty must be a whole number of contracts, got {q}"));
            }
            if q > abs {
                return Err(format!(
                    "qty {q} exceeds |position| {abs} — a close never flips a position"
                ));
            }
            q
        }
    };
    Ok((side, close_qty))
}

/// Pure, FROZEN seam (ADR 0022 §3): the 10-key option-close ack. Echoes the RESOLVED row
/// identity (`conid`/`symbol`/`expiry`/`strike`/`right` — from the matched position, not
/// user input), the DERIVED `action` (sign of held position), `quantity` (derived), and
/// `limit_price` (always a number — LMT-only). `order_id`/`status` from allocation + first ack.
#[allow(clippy::too_many_arguments)] // the 10-key ack shape IS the frozen contract (brief.rs:27 precedent)
pub fn shape_option_close_ack(
    order_id: i32,
    status: &str,
    conid: i32,
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
        "conid": conid,
        "symbol": symbol,
        "expiry": expiry,
        "strike": strike,
        "right": right,
        "action": action,
        "quantity": quantity,
        "limit_price": limit_price,
    })
}

/// Pure, FROZEN seam (ADR 0023 §3): working orders that BLOCK a close on `conid` for a
/// position of this sign = SAME conid AND action OPPOSITE to the position. Long (>0) ⇒
/// "Sell" blocks; short (<0) ⇒ "Buy" blocks; same-side orders (adds) never block; other
/// conids never block. `position == 0.0` ⇒ empty (defensive totality — the verb refuses
/// flat positions upstream). Action strings are the Debug forms the shared drain emits
/// ("Buy"/"Sell", orders.rs precedent). Returns ids ASCENDING (deterministic output).
pub fn blocking_close_order_ids(
    position: f64,
    conid: i32,
    open_orders: &[(i32, i32, String)],
) -> Vec<i32> {
    let opposite = if position > 0.0 {
        "Sell"
    } else if position < 0.0 {
        "Buy"
    } else {
        // Flat ⇒ nothing blocks (unreachable in the verb; seam stays total).
        return Vec::new();
    };
    let mut ids: Vec<i32> = open_orders
        .iter()
        .filter(|(_, c, a)| *c == conid && a == opposite)
        .map(|(id, _, _)| *id)
        .collect();
    ids.sort_unstable();
    ids
}


/// Pure, FROZEN seam (ADR 0024 §2): stamp the resolved account onto an order. Sets
/// `order.account` (OVERWRITING any prior value — the resolved account is the ONLY
/// authority) and touches nothing else. Called at the single placement choke point
/// (`place_with_client`) so no current or future verb can skip it.
pub fn stamp_order_account(order: &mut Order, account: &str) {
    order.account = account.to_string();
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

/// Placement body (ADR 0020 D7 + review-01 fix): everything from `next_order_id()` onward,
/// taking an already-connected client. Called by `place_core` (stk/option single-leg) and
/// directly by `option_combo` (which needs the client for per-leg conid resolution first —
/// never a second same-client-id connect).
fn place_with_client(
    client: &Client,
    ctx: &str,
    contract: &Contract,
    order: &Order,
    account: &AccountId,
    ack: impl Fn(i32, &str) -> Value,
) -> Result<Value, AppError> {
    // Stamp the resolved account onto the order BEFORE placement (ADR 0024). Clone then
    // stamp — every placement path funnels through this choke point, so no current or
    // future verb can skip it. The builders' pure output (account="") is untouched.
    let mut order = order.clone();
    stamp_order_account(&mut order, &account.0);

    // Allocate the order id FIRST so even a timeout error can NAME it. Uses the
    // handshake-seeded local allocator (client.next_order_id, ADR 0018): non-blocking,
    // returns the id_manager's next id. The prior next_valid_order_id() was an unbounded
    // subscription.next() that this gateway never answers (paper wedge, ADR 0018).
    let order_id = client.next_order_id();

    // Place.
    let subscription = client
        .place_order(order_id, contract, &order)
        .map_err(|e| AppError::data(format!("place_order failed: {e}"), ctx))?;

    // Bounded first-ack loop (ADR 0016 bounded-iterator pattern). Take the FIRST
    // OrderStatus or OpenOrder event; skip ExecutionData/CommissionReport (window
    // refreshes on each arrival). Any None before an ack = UNKNOWN state (the order
    // MAY have been submitted) — never a silent success, never a blind retry.
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

/// Preview placement (ADR 0026, gateway fn — review-by-reading + operator
/// live-acceptance, NOT frozen). Identical to `place_with_client` with two deltas:
///   1. after the account stamp, set `order.what_if = true` (the non-transmitting
///      whatIf query flag);
///   2. in the bounded first-ack loop, return `shape_preview(contract, &order,
///      &od.order_state)` on the FIRST `OpenOrder(od)` — the whatIf margin/commission
///      payload rides `OpenOrder.order_state` (skip `OrderStatus`/ExecutionData/etc).
///
/// Same bounded first-ack loop + UNKNOWN semantics. For preview UNKNOWN = "no preview
/// data"; nothing transmitted, assuming Tiger honors `what_if` (CONTEXT.md R1).
fn preview_with_client(
    client: &Client,
    ctx: &str,
    contract: &Contract,
    order: &Order,
    account: &AccountId,
) -> Result<Value, AppError> {
    let mut order = order.clone();
    stamp_order_account(&mut order, &account.0);
    order.what_if = true;

    let order_id = client.next_order_id();

    let subscription = client
        .place_order(order_id, contract, &order)
        .map_err(|e| AppError::data(format!("place_order failed: {e}"), ctx))?;

    let mut items = subscription.timeout_iter_data(super::TAKE_FIRST_TIMEOUT);
    loop {
        match items.next() {
            Some(Ok(PlaceOrder::OpenOrder(od))) => {
                return Ok(shape_preview(contract, &order, &od.order_state));
            }
            Some(Ok(_)) => {} // OrderStatus / ExecutionData / CommissionReport — skip until OpenOrder.
            Some(Err(e)) => {
                return Err(AppError::data(
                    format!("order stream: {e}"),
                    ctx,
                ))
            }
            None => {
                return Err(AppError::timeout(
                    format!(
                        "preview order {order_id} — no OpenOrder within {}s; no preview data \
                         available (nothing transmitted assuming Tiger honors whatIf)",
                        super::TAKE_FIRST_TIMEOUT.as_secs()
                    ),
                    ctx,
                ))
            }
        }
    }
}

/// Shared placement core (stk + single-leg option). Thin wrapper: gate → connect →
/// `place_with_client`. Behavior byte-identical to the pre-refactor stk path — the frozen
/// stk + option-orders suites assert it. `option_combo` calls `place_with_client` directly
/// (it has its own client for per-leg conid resolution — never a second connect).
fn place_core(
    cfg: &Config,
    ctx: &str,
    contract: &Contract,
    order: &Order,
    ack: impl Fn(i32, &str) -> Value,
) -> Result<Value, AppError> {
    require_live_write_gate(cfg)?;
    let client = super::connect(cfg)?;
    let account = super::resolve_account(&client, cfg)?;
    if cfg.preview {
        preview_with_client(&client, ctx, contract, order, &account)
    } else {
        place_with_client(&client, ctx, contract, order, &account, ack)
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

// ---------------------------------------------------------------------------
// Combo (BAG) orders — ADR 0021
// ---------------------------------------------------------------------------

/// Pure, FROZEN seam: one combo leg spec parsed from the 6-token DSL
/// "ACTION RATIO SYMBOL EXPIRY STRIKE RIGHT" (e.g. "BUY 1 AAPL 20260918 240 C").
/// All fields normalized: action uppercase, symbol uppercase, right "C"|"P".
#[derive(Debug, PartialEq)]
pub struct LegSpec {
    pub action: String,
    pub ratio: i32,
    pub symbol: String,
    pub expiry: String,
    pub strike: f64,
    pub right: String,
}

/// Pure, FROZEN seam: parse one leg DSL string → LegSpec. Returns Err(reason) for any
/// malformed token (wrong count, bad action, ratio <= 0 or non-integer, bad expiry shape,
/// strike not finite-positive, bad right). The gateway fn wraps the Err as usage naming
/// "leg N: reason".
pub fn parse_combo_leg(s: &str) -> Result<LegSpec, String> {
    let tokens: Vec<&str> = s.split_whitespace().collect();
    if tokens.len() != 6 {
        return Err(format!("expected 6 tokens, got {}", tokens.len()));
    }
    let action = tokens[0].to_ascii_uppercase();
    if action != "BUY" && action != "SELL" {
        return Err(format!("invalid action '{}': expected BUY or SELL", tokens[0]));
    }
    let ratio: i32 = tokens[1].parse().map_err(|_| format!("invalid ratio '{}': must be an integer", tokens[1]))?;
    if ratio <= 0 {
        return Err(format!("ratio must be > 0, got {}", ratio));
    }
    let symbol = tokens[2].to_ascii_uppercase();
    let expiry = tokens[3].to_string();
    super::option_quote::parse_expiry(&expiry)
        .ok_or_else(|| format!("invalid expiry '{}': expected 8-digit YYYYMMDD", tokens[3]))?;
    let strike: f64 = tokens[4].parse().map_err(|_| format!("invalid strike '{}': must be a number", tokens[4]))?;
    if !strike.is_finite() || strike <= 0.0 {
        return Err(format!("strike must be finite-positive, got {}", strike));
    }
    let right = super::option_quote::normalize_right(tokens[5])
        .ok_or_else(|| format!("invalid right '{}': expected C|CALL or P|PUT", tokens[5]))?
        .to_string();
    Ok(LegSpec { action, ratio, symbol, expiry, strike, right })
}

/// Pure, FROZEN seam: validated leg specs → the exact `(Contract, Order)` for a BAG.
/// Contract via SpreadBuilder (add_leg per spec in input order with ratio + LegAction);
/// `.on_exchange` / `.in_currency` applied to the spread; `.build()?`; **back-fill**
/// `contract.symbol = underlying` (SpreadBuilder leaves it empty). Order is LMT with
/// **SIGN-FREE** net limit (negative = credit — deliberately unlike single-leg) / DAY.
pub fn build_combo_order(
    underlying: &str,
    legs: &[(&LegSpec, i32)],
    side: Action,
    quantity: f64,
    limit: f64,
    exchange: &str,
    currency: &str,
) -> Result<(Contract, Order), String> {
    let mut builder = Contract::spread();
    for (spec, conid) in legs {
        let leg_action = if spec.action == "BUY" { LegAction::Buy } else { LegAction::Sell };
        builder = builder.add_leg(*conid, leg_action).ratio(spec.ratio).done();
    }
    let mut contract = builder
        .on_exchange(exchange)
        .in_currency(currency)
        .build()
        .map_err(|e| format!("spread build failed: {e}"))?;
    // Back-fill the underlying symbol — SpreadBuilder leaves it as Default.
    contract.symbol = underlying.into();
    let order = Order {
        action: side,
        total_quantity: quantity,
        order_type: "LMT".into(),
        limit_price: Some(limit), // SIGN-FREE: negative = credit
        tif: TimeInForce::Day,
        ..Default::default()
    };
    Ok((contract, order))
}

/// Pure, FROZEN seam: the combo ack — 7 top-level keys + a `legs` array of 7-key echoes.
/// Each leg echo carries the spec fields + the resolved conid, in input order.
#[allow(clippy::too_many_arguments)] // the shape IS the frozen contract (brief.rs:27 precedent)
pub fn shape_combo_order_ack(
    order_id: i32,
    status: &str,
    underlying: &str,
    action: &str,
    quantity: f64,
    limit_price: f64,
    legs: &[(&LegSpec, i32)],
) -> Value {
    json!({
        "order_id": order_id,
        "status": status,
        "underlying": underlying,
        "action": action,
        "quantity": quantity,
        "limit_price": limit_price,
        "legs": legs
            .iter()
            .map(|(spec, conid)| {
                json!({
                    "action": spec.action,
                    "ratio": spec.ratio,
                    "symbol": spec.symbol,
                    "expiry": spec.expiry,
                    "strike": spec.strike,
                    "right": spec.right,
                    "conid": conid,
                })
            })
            .collect::<Vec<_>>(),
    })
}

/// Place a multi-leg option combo (BAG) order. LMT/DAY only, net limit sign-free.
/// Validation < gate < connect ordering frozen; leg errors name the 1-based index.
pub fn option_combo(cfg: &Config, args: &OptionComboArgs) -> Result<Value, AppError> {
    let ctx = "option-combo";

    // 1. Local validation (usage errors, before gate and connect). Ordering frozen:
    //    usage < config(gate) < connection.
    let side = match args.action.to_ascii_lowercase().as_str() {
        "buy" => Action::Buy,
        "sell" => Action::Sell,
        _ => {
            return Err(AppError::usage(
                format!("invalid --action {}: expected BUY or SELL", args.action),
                ctx,
            ));
        }
    };
    if args.legs.len() < 2 || args.legs.len() > 4 {
 return Err(AppError::usage(
            format!("--leg count must be 2..=4, got {}", args.legs.len()),
            ctx,
        ));
    }
    // Parse legs (1-based index in error messages).
    let mut specs: Vec<LegSpec> = Vec::with_capacity(args.legs.len());
    for (i, raw) in args.legs.iter().enumerate() {
        let n = i + 1;
        let spec = parse_combo_leg(raw).map_err(|reason| {
            AppError::usage(format!("leg {n}: {reason}"), ctx)
        })?;
        specs.push(spec);
    }
    // Same-symbol rule: all legs must share the underlying.
    let underlying = specs[0].symbol.clone();
    if !specs.iter().all(|s| s.symbol == underlying) {
        return Err(AppError::usage(
            "all legs must share the same underlying symbol".to_string(),
            ctx,
        ));
    }
    if !args.qty.is_finite() || args.qty < 1.0 || args.qty.fract() != 0.0 {
        return Err(AppError::usage(
            format!("--qty must be a whole number of contracts >= 1 (got {})", args.qty),
            ctx,
        ));
    }
    if !args.limit.is_finite() {
        return Err(AppError::usage(
            format!("--limit must be finite (got {})", args.limit),
            ctx,
        ));
    }

    // 2. Double gate (config error, before connect — offline-deterministic).
    require_live_write_gate(cfg)?;

    // 3. Connect (connection errors).
    let client = super::connect(cfg)?;

    // Resolve the account on the SAME client (single bounded read) — passed to the
    // placement choke point so the order is stamped (ADR 0024).
    let account = super::resolve_account(&client, cfg)?;

    // 4. Per-leg conid resolve (fail-fast, naming leg N). Each leg is an option contract
    //    resolved via contract_details FIRST row (ADR 0019 D4 parity).
    let exchange = &args.exchange;
    let currency = &args.currency;
    let mut resolved: Vec<(LegSpec, i32)> = Vec::with_capacity(specs.len());
    for (i, spec) in specs.into_iter().enumerate() {
        let n = i + 1;
        let builder = match spec.right.as_str() {
            "C" => Contract::call(&spec.symbol),
            _ => Contract::put(&spec.symbol),
        }
        .strike(spec.strike);
        let (y, m, d) = super::option_quote::parse_expiry(&spec.expiry)
            .ok_or_else(|| AppError::usage(format!("leg {n}: internal: unparseable expiry"), ctx))?;
        let leg_contract = builder
            .expires_on(y, m, d)
            .on_exchange(exchange)
            .in_currency(currency)
            .build();
        let details = client
            .contract_details(&leg_contract)
            .map_err(|e| AppError::data(format!("leg {n}: contract_details failed: {e}"), ctx))?;
        let conid = details.first().map(|d| d.contract.contract_id).ok_or_else(|| {
            AppError::not_found(format!("leg {n}: no contract for {} {} {}", spec.symbol, spec.strike, spec.right), ctx)
        })?;
        resolved.push((spec, conid));
    }

    // 5. Build + place via the shared core.
    let leg_refs: Vec<(&LegSpec, i32)> = resolved.iter().map(|(s, c)| (s, *c)).collect();
    let (contract, order) = build_combo_order(
        &underlying,
        &leg_refs,
        side,
        args.qty,
        args.limit,
        exchange,
        currency,
    )
    .map_err(|reason| AppError::usage(format!("build_combo_order: {reason}"), ctx))?;

    let action_str = format!("{:?}", side);
    let legs_snapshot: Vec<(LegSpec, i32)> = resolved;
    if cfg.preview {
        preview_with_client(&client, ctx, &contract, &order, &account)
    } else {
        place_with_client(&client, ctx, &contract, &order, &account, |id, status| {
            let leg_refs: Vec<(&LegSpec, i32)> = legs_snapshot.iter().map(|(s, c)| (s, *c)).collect();
            shape_combo_order_ack(id, status, &underlying, &action_str, args.qty, args.limit, &leg_refs)
        })
    }
}

// ---------------------------------------------------------------------------
// Option-close (close-by-conid) — ADR 0022
// ---------------------------------------------------------------------------

/// Close a HELD option position by conid (`omi option-close`). LMT/DAY only, side DERIVED
/// from the held position's sign (long ⇒ SELL, short ⇒ BUY — never user-declared; anti-double
/// gate). Single-connect: drain → match → rebuild → conid assert → place all on ONE client
/// (a second same-client-id connect wedges the gateway — option-combo review lesson).
/// Validation ordering frozen: usage (local) < config (gate) < connection.
pub fn option_close(cfg: &Config, args: &OptionCloseArgs) -> Result<Value, AppError> {
    let ctx = "option-close";

    // 1. Local validation (usage errors, before gate and connect). Ordering frozen:
    //    usage < config(gate) < connection.
    if args.conid < 1 {
        return Err(AppError::usage(
            format!("--conid must be a positive integer (got {})", args.conid),
            ctx,
        ));
    }
    if !args.limit.is_finite() || args.limit <= 0.0 {
        return Err(AppError::usage(
            format!("--limit must be a finite positive number (got {})", args.limit),
            ctx,
        ));
    }
    if let Some(q) = args.qty {
        if !q.is_finite() {
            return Err(AppError::usage(
                format!("--qty must be finite, got {q}"),
                ctx,
            ));
        }
        if q < 1.0 {
            return Err(AppError::usage(
                format!("--qty must be a whole number of contracts >= 1 (got {q})"),
                ctx,
            ));
        }
        if q.fract() != 0.0 {
            return Err(AppError::usage(
                format!("--qty must be a whole number of contracts (got {q})"),
                ctx,
            ));
        }
    }

    // 2. Double gate (config error, before connect — offline-deterministic).
    require_live_write_gate(cfg)?;

    // 3. Connect ONCE — drain, resolve, assert, and place all reuse this client.
    let client = super::connect(cfg)?;

    // 4. Account.
    let account = super::resolve_account(&client, cfg)?;

    // 5. Drain account_updates to End; LAST row whose conid matches wins (latest snapshot).
    //    Reuses the End-marker pattern from positions() (ADR 0011).
    let subscription = client
        .account_updates(&account)
        .map_err(|e| AppError::data(format!("account_updates failed: {e}"), ctx))?;
    let mut matched: Option<AccountPortfolioValue> = None;
    for update in subscription.iter_data() {
        let update = update
            .map_err(|e| AppError::data(format!("account_updates stream: {e}"), ctx))?;
        match update {
            AccountUpdate::PortfolioValue(p) => {
                if p.contract.contract_id == args.conid {
                    matched = Some(p); // last match wins
                }
            }
            AccountUpdate::End => break,
            _ => {}
        }
    }

    // Anti-open gate: refuse if not held or already flat.
    let row = matched.ok_or_else(|| {
        AppError::not_found(
            format!(
                "no open position for conid {} — nothing to close; see `omi positions`",
                args.conid
            ),
            ctx,
        )
    })?;
    if row.position == 0.0 {
        return Err(AppError::not_found(
            format!(
                "position for conid {} is 0 — nothing to close; see `omi positions`",
                args.conid
            ),
            ctx,
        ));
    }
    // Non-OPT conid ⇒ usage (this is an option verb — point at the stock verbs).
    if !matches!(row.contract.security_type, SecurityType::Option) {
        return Err(AppError::usage(
            format!(
                "conid {} is {}, not an option — use `omi sell`/`omi buy` for stock",
                args.conid,
                row.contract.security_type
            ),
            ctx,
        ));
    }

    // 5.5 Pending-close guard (ADR 0023): scan working orders ON THE SAME CLIENT; if any
    //     same-conid order has action OPPOSITE to the position sign, refuse — naming every
    //     blocking id. Kills the double-fire path (retry after a placement timeout). Zero
    //     new connects: reuses the shared drain already proven on the read path.
    let open_orders_value = super::orders::open_orders_with_client(&client, None, ctx)?;
    let open_rows = open_orders_value.as_array().ok_or_else(|| {
        AppError::data(
            format!(
                "open_orders drain did not return an array for conid {} — refusing to place",
                args.conid
            ),
            ctx,
        )
    })?;
    // Extract (order_id, conid, action) triples; a malformed row ⇒ data error naming the
    // index (NEVER skip — a skipped row could hide a blocker; fail-closed, ADR 0023 §5).
    let mut triples: Vec<(i32, i32, String)> = Vec::with_capacity(open_rows.len());
    for (i, r) in open_rows.iter().enumerate() {
        let order_id = r["order_id"].as_i64().ok_or_else(|| {
            AppError::data(
                format!("open_orders row {i}: missing/invalid order_id — refusing to place"),
                ctx,
            )
        })?;
        let row_conid = r["conid"].as_i64().ok_or_else(|| {
            AppError::data(
                format!("open_orders row {i}: missing/invalid conid — refusing to place"),
                ctx,
            )
        })?;
        let action = r["action"].as_str().ok_or_else(|| {
            AppError::data(
                format!("open_orders row {i}: missing/invalid action — refusing to place"),
                ctx,
            )
        })?;
        triples.push((order_id as i32, row_conid as i32, action.to_string()));
    }
    let blocking = blocking_close_order_ids(row.position, args.conid, &triples);
    if !blocking.is_empty() {
        let ids = blocking
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        return Err(AppError::not_found(
            format!(
                "close blocked: working close order(s) [{ids}] already cover conid {} — \
                 cancel first (`omi cancel <id>`) or inspect `omi orders`; a second close \
                 would flip the position",
                args.conid
            ),
            ctx,
        ));
    }
    // 6. Derive close side + qty from the SIGN of the held position (anti-double gate).
    let (side, close_qty) =
        derive_close(row.position, args.qty).map_err(|reason| AppError::usage(reason, ctx))?;

    // 7. Rebuild via the live-proven builder chain (ADR 0020 D8 VERBATIM reuse).
    let symbol = row.contract.symbol.to_string();
    let raw_expiry = row.contract.last_trade_date_or_contract_month.clone();
    let (y, m, d) = super::option_quote::parse_expiry(&raw_expiry).ok_or_else(|| {
        AppError::data(
            format!("expiry unparseable for conid {}: got {raw_expiry:?}", args.conid),
            ctx,
        )
    })?;
    let strike = row.contract.strike;
    let right = match row.contract.right {
        Some(OptionRight::Call) => OptionRight::Call,
        Some(OptionRight::Put) => OptionRight::Put,
        // None or any non_exhaustive future variant ⇒ can't rebuild an option contract.
        _ => {
            return Err(AppError::data(
                format!(
                    "conid {}: held option has no recognizable right (got {:?})",
                    args.conid, row.contract.right
                ),
                ctx,
            ));
        }
    };
    let right_str = match right {
        OptionRight::Call => "C",
        _ => "P",
    };
    let trading_class = if row.contract.trading_class.is_empty() {
        None
    } else {
        Some(row.contract.trading_class.as_str())
    };
    let currency = if row.contract.currency.as_str().is_empty() {
        "USD"
    } else {
        row.contract.currency.as_str()
    };
    let (contract, order) = build_option_order(
        &symbol,
        (y, m, d),
        strike,
        right,
        trading_class,
        "SMART",
        currency,
        side,
        close_qty,
        args.limit,
    );

    // 8. Wrong-contract gate (ADR 0021): the resolved contract's conid MUST match the
    //    requested one BEFORE any placement. Refuses on mismatch (data — never places).
    let details = client.contract_details(&contract).map_err(|e| {
        AppError::data(format!("contract_details failed: {e}"), ctx)
    })?;
    let resolved_conid = details.first().map(|d| d.contract.contract_id).ok_or_else(|| {
        AppError::not_found(
            format!("no contract resolved for conid {} — refusing to place", args.conid),
            ctx,
        )
    })?;
    if resolved_conid != args.conid {
        return Err(AppError::data(
            format!(
                "resolved conid {resolved_conid} != requested {} — refusing to place",
                args.conid
            ),
            ctx,
        ));
    }

    // 9. Place via the shared body (allocate → bounded first-ack → no retry). The ack echoes
    //    the RESOLVED row identity; `action` is the DERIVED side.
    let action_str = format!("{:?}", side);
    let expiry_echo = raw_expiry.clone();
    if cfg.preview {
        preview_with_client(&client, ctx, &contract, &order, &account)
    } else {
        place_with_client(&client, ctx, &contract, &order, &account, |id, status| {
            shape_option_close_ack(
                id,
                status,
                args.conid,
                &symbol,
                &expiry_echo,
                strike,
                right_str,
                &action_str,
                close_qty,
                args.limit,
            )
        })
    }
}
