# arch — stk-orders (Phase 2 opener: the write path)

Binding decisions in **ADR 0017**; glossary in `CONTEXT.md`. Every ibapi claim source-verified.
HIGHEST-STAKES feature: review polarity flips — writes must exist ONLY in the new module and
be unreachable from every read command.

## Design shape (five touched files + two doc amendments, no new dependency)

| file | change |
|---|---|
| `src/cli.rs` | `Buy(OrderArgs)`, `Sell(OrderArgs)`, `Cancel(CancelArgs)`; `OrderArgs { symbol: String, quantity: f64, #[arg(long)] limit: Option<f64> }`; `CancelArgs { order_id: i32 }` |
| `src/ib/trade.rs` | NEW — the ONLY file with write calls: pure seams `build_stk_order` + `shape_order_ack`, gate helper `require_live_write_gate`, gateway fns `buy`/`sell`/`cancel` |
| `src/ib/mod.rs` | `mod trade;` + re-exports (`buy, sell, cancel, build_stk_order, shape_order_ack, require_live_write_gate`) |
| `src/main.rs` | three dispatch arms |
| `AGENTS.md` + `CLAUDE.md` | red-line paragraph amendment — VERBATIM text in §Docs amendment below, nothing else |

NOT touched: every read module, `output.rs`, `error.rs`, `config.rs` (gate reads env at call
time — no config surface), all tests.

## ibapi facts (source-verified, 2026-07-03)

- `next_valid_order_id() -> Result<i32>` (orders/sync.rs:192-206) — bounded request-response,
  crate-managed; allocate BEFORE placing so even a timeout error can NAME the order id.
- `place_order(order_id, &Contract, &Order) -> Result<Subscription<PlaceOrder>, Error>`
  (sync.rs:271-279) — order-id-routed subscription; events: `OrderStatus(OrderStatus)` /
  `OpenOrder(OrderData)` / `ExecutionData` / `CommissionReport` (mod.rs:1519-1528).
  (`submit_order` is fire-and-forget — rejected by PRD D4.)
- `cancel_order(order_id, "") -> Result<Subscription<CancelOrder>, Error>` (sync.rs:77-89);
  `CancelOrder::OrderStatus(OrderStatus)` is the only variant (mod.rs:1578-1581).
- `OrderStatus` carries `order_id`, `status: OrderStatusKind`, `filled`, `remaining`,
  `average_fill_price: Option<f64>` (mod.rs:1549-1561).
- Order construction: plain `Order` struct fields (action, total_quantity, order_type
  "LMT"/"MKT", limit_price, tif Day) — the builder exists but a direct struct keeps the seam
  pure and frozen-testable (tests/ may use ibapi types — `quote_ticks.rs` precedent).
- Routing: `send_order(order_id, …)` = the order-id domain (disjoint from request-id/shared —
  ADR 0010 table), so place/cancel subscriptions cannot cross-talk with anything else.

## Component design (impl follows verbatim)

`src/ib/trade.rs`:

```rust
/// Pure, FROZEN seam: CLI params → the exact (Contract, Order) pair sent to the gateway.
/// side: Action::Buy | Action::Sell. limit None ⇒ MKT, Some ⇒ LMT. TIF always Day (v1).
pub fn build_stk_order(symbol: &str, side: Action, quantity: f64, limit: Option<f64>)
    -> (Contract, Order)
{
    let contract = Contract::stock(symbol).build();   // SMART/USD defaults, quote parity
    let mut order = Order { action: side, total_quantity: quantity, tif: TimeInForce::Day, ..Default::default() };
    match limit {
        Some(px) => { order.order_type = "LMT".into(); order.limit_price = Some(px); }
        None => { order.order_type = "MKT".into(); }
    }
    (contract, order)
}

/// Pure, FROZEN seam: ack JSON. Echo fields are deterministic (from the request);
/// order_id/status come from allocation + the first ack event.
pub fn shape_order_ack(order_id: i32, status: &str, symbol: &str, action: &str,
    quantity: f64, limit_price: Option<f64>) -> Value
{ json!({ "order_id", "status", "symbol", "action", "quantity", "limit_price" /* null for MKT */ }) }

/// The double gate (ADR 0017). MUST run before super::connect — offline-deterministic.
/// Gate on the EFFECTIVE live port (covers both `--live` and a hand-set `--port 4001`).
pub fn require_live_write_gate(cfg: &Config) -> Result<(), AppError> {
    if cfg.port == crate::config::LIVE_PORT && std::env::var("OMI_ALLOW_LIVE").as_deref() != Ok("1") {
        return Err(AppError::config(
            "live order rejected: set OMI_ALLOW_LIVE=1 to enable live trading (paper :4002 needs no gate)",
            "live write gate",
        ));
    }
    Ok(())
}
```

Gateway fns (review-by-reading; all waits bounded by `TAKE_FIRST_TIMEOUT`, reuse — no new
consts):

- `buy(cfg, args)` / `sell(cfg, args)`: local validation (quantity > 0 else usage; limit > 0
  when present else usage) → `require_live_write_gate` → connect → `next_valid_order_id` →
  `build_stk_order` → `place_order` → **bounded first-ack loop**: iterate
  `timeout_iter_data(TAKE_FIRST_TIMEOUT)` (the ADR 0016 Instant-classified pattern), take the
  FIRST `OrderStatus` (Debug-render its `status`) or `OpenOrder` (render
  `order_state.status`); skip ExecutionData/CommissionReport while the window refreshes;
  timeout ⇒ `AppError::timeout("order {id} may have been SUBMITTED — no ack within 10s; verify
  with `omi orders`, do NOT retry blindly", "buy"|"sell")`; stream-ended-instant `None` before
  any ack ⇒ same timeout error (UNKNOWN state) — never a silent success.
  Success ⇒ `shape_order_ack(...)`.
- `cancel(cfg, args)`: gate → connect → `cancel_order(order_id, "")` → bounded first
  `CancelOrder::OrderStatus` ⇒ `{order_id, status}`; timeout ⇒ timeout envelope naming
  `omi orders` verification.
- NO retry anywhere. NO order placement outside `trade.rs` (review greps for
  `place_order|submit_order|encode_place_order` — hits only in trade.rs).

## Docs amendment (impl copies VERBATIM)

`AGENTS.md`/`CLAUDE.md` — replace the read-only red-line bullet with:

> - **Writes are gated** — Phase 2 (2026-07-03) added `buy`/`sell`/`cancel` (STK, LMT/MKT,
>   DAY). Paper (`:4002`, the default) is ungated; **live orders require BOTH `--live` AND
>   `OMI_ALLOW_LIVE=1`**. All other commands remain read-only; no modify, no options, no
>   combos yet. Write code lives ONLY in `src/ib/trade.rs`.

## Freeze coverage (pinned for pipeline-task)

- **Frozen (`tests/stk_orders_command.rs`, offline):**
  - `build_stk_order`: LMT (action/qty/limit_price/order_type/tif Day) and MKT (no limit
    price, order_type MKT) for both sides — asserting ibapi `Order` fields directly
    (quote_ticks.rs precedent).
  - `shape_order_ack`: exact 6-key set; MKT ⇒ `limit_price: null`.
  - **Gate matrix (offline — gate precedes connect):** for each of buy/sell/cancel:
    `--live` without env ⇒ `code="config"`, non-zero exit; `--port 4001` without env ⇒ same
    (effective-port rule); with `OMI_ALLOW_LIVE=1` + `--live` + dead port ⇒
    `code="connection"` (gate passed). Paper default + dead port ⇒ `code="connection"`
    (no gate in the way).
  - Validation: qty 0/negative ⇒ usage; `--limit 0` ⇒ usage; missing args ⇒ usage.
  - `--help` lists buy/sell/cancel.
- **Review-by-reading:** gateway fns (bounded loops, no-retry, ack-event choice, UNKNOWN-state
  message); the polarity grep (write calls ONLY in trade.rs, unreachable from reads); the
  docs amendment verbatim-match.
- **Live (operator, PAPER `:4002`, PRD criterion 11):** place far-LMT buy → `orders` shows →
  `cancel` → `completed-orders` Cancelled → `positions` unchanged.

## Risks re-checked

- First ack may be `OpenOrder` before `OrderStatus` depending on gateway — both accepted as
  the ack (design above), so ordering variance is safe.
- MKT on paper outside market hours may sit PreSubmitted — acceptance uses far-LMT, and
  PreSubmitted IS a valid ack status.
- The env var is read per-invocation (no caching); `assert_cmd` `.env()`/`.env_remove()`
  drives the frozen matrix deterministically.
- Wedge dossier: order-id routing domain is new territory on this gateway — every wait
  bounded; worst case is the UNKNOWN-state envelope, never a hang, never a blind retry.
