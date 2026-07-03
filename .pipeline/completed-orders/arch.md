# arch — completed-orders

How `omi completed-orders` lands. Binding decisions in **ADR 0015**; glossary in `CONTEXT.md`.
All ibapi claims verified in the vendored crate source.

## Design shape (four touched files, no new dependency)

| file | change |
|---|---|
| `src/cli.rs` | `CompletedOrders` variant (doc: "Today's completed orders (filled/cancelled) with status") — no args beyond globals |
| `src/ib/completed_orders.rs` | NEW — plain `CompletedOrderRow`, pure frozen seam `shape_completed_orders`, gateway drain fn `completed_orders` |
| `src/ib/mod.rs` | `mod completed_orders;` + `pub use completed_orders::{completed_orders, shape_completed_orders, CompletedOrderRow};` |
| `src/main.rs` | `Command::CompletedOrders => ib::completed_orders(&config),` |

NOT touched: `orders.rs` (sibling, mirrored not modified), `output.rs`, `error.rs`, `brief.rs`
(brief's 8-key top level is FROZEN — no new section), all tests.

## ibapi facts (source-verified, 2026-07-03)

- `client.completed_orders(api_only: bool) -> Result<Subscription<Orders>, Error>`
  (orders/sync.rs:108-115) — shared-channel request whose response set is
  `[CompletedOrder, CompletedOrdersEnd]` (messages/shared_channel_configuration.rs:45): the
  subscription's `iter_data()` terminates itself on the End message — the `orders.rs` drain
  pattern VERBATIM (no explicit break, no markerless hazard, no ADR 0012 timeout).
- Row sources: `Orders::OrderData(d)` where `d.order_id`, `d.contract` (symbol/contract_id),
  `d.order` (account, action, total_quantity, order_type, limit_price `Option<f64>`,
  aux_price `Option<f64>`, tif, **filled_quantity: f64** — orders/mod.rs:391),
  `d.order_state` (**status: OrderStatusKind** mod.rs:1277, **completed_time: String**
  mod.rs:1333, **completed_status: String** mod.rs:1336).
- `OrderStatusKind` renders via `format!("{:?}")` (Filled / Cancelled / …) — same Debug-render
  convention orders.rs uses for `action`/`tif`.
- `Orders::OrderStatus(_)` variants are skipped (OrderData-only arm) — orders.rs posture.

## Component design (impl follows this verbatim)

`src/ib/completed_orders.rs`:

```rust
/// Plain, ibapi-free completed-order row (the frozen test constructs these directly).
pub struct CompletedOrderRow {
    pub order_id: i32,
    pub account: String,
    pub symbol: String,
    pub conid: i32,
    pub action: String,          // Debug-rendered Action
    pub quantity: f64,           // order.total_quantity
    pub order_type: String,
    pub limit_price: Option<f64>,   // None → null (raw pass-through, orders.rs parity)
    pub aux_price: Option<f64>,
    pub tif: String,             // Debug-rendered
    pub status: String,          // Debug-rendered OrderStatusKind (Filled/Cancelled/…)
    pub filled_quantity: f64,
    pub completed_time: String,  // "" when the gateway omits it
    pub completed_status: String,
}

/// The pure, FROZEN seam: rows in gateway order → JSON array of exact 14-key objects
/// (open-orders 10-key parity + 4 completion keys). Empty ⇒ json!([]).
pub fn shape_completed_orders(rows: Vec<CompletedOrderRow>) -> Value { … }

pub fn completed_orders(cfg: &Config) -> Result<Value, AppError> {
    let client = super::connect(cfg)?;
    let subscription = client.completed_orders(false)   // D4: api_only=false, hardcoded
        .map_err(|e| AppError::data(format!("completed_orders failed: {e}"), "completed-orders"))?;
    let mut rows = Vec::new();
    for item in subscription.iter_data() {
        let item = item.map_err(|e| AppError::data(format!("completed orders stream: {e}"), "completed-orders"))?;
        if let Orders::OrderData(d) = item {
            if let Some(acct) = cfg.account.as_deref() {      // D5: filter ONLY when --account set
                if d.order.account != acct { continue; }
            }
            rows.push(CompletedOrderRow { /* field mapping per §ibapi facts */ });
        }
    }
    Ok(json!({ "completed_orders": shape_completed_orders(rows) }))
}
```

Filter parity note: `orders.rs` threads `account_filter` through a `_with_client` seam because
`brief` shares it; completed-orders has NO brief consumer (non-scope), so the filter lives
inline in the single gateway fn — same SEMANTICS (`cfg.account` set ⇒ filter; unset ⇒
pass-through), simpler shape. Review checks semantics parity, not code-shape identity.

## Freeze coverage (pinned for pipeline-task)

- **Frozen (`tests/completed_orders_command.rs`):** `shape_completed_orders` (exact 14-key
  row; gateway order preserved; `limit_price/aux_price` None → null; "" completed_time passes
  through; zero rows ⇒ `[]`); CLI (`--help` lists `completed-orders`;
  `completed-orders --help` ok; dead port ⇒ `code="connection"`). House-red via
  `use oh_my_ib::ib::{shape_completed_orders, CompletedOrderRow};`.
- **Review-by-reading:** the drain fn — `completed_orders(false)` call, OrderData-only arm,
  filter-when-set semantics vs `orders.rs`, `{"completed_orders": …}` wrapper, contexts
  `completed-orders`. READ-ONLY red line: grep the diff for place/cancel/modify — must be absent.
- **Live (operator, PRD criterion 8, merge gate):** `omi --live completed-orders` exits 0
  with the wrapper shape; `[]` is a PASS on a no-trade day (row content rides the first
  active trading day).

## Risks re-checked

- Server-version variance on completion fields: decoder defaults yield ""/0 — the row is
  total; frozen test pins pass-through.
- `average_fill_price` lives on `Orders::OrderStatus`, NOT on OrderData — deliberately out
  (executions carries fill prices); recorded so impl doesn't chase it.
- Rollback: additive subcommand.

## Amendment (2026-07-03, post live-wedge) — bounded drain (ADR 0016)

Live acceptance hung twice on a healthy gateway (`CompletedOrdersEnd` never arrived — known
upstream class, see ADR 0016). §Component design's drain loop is REPLACED by:

```rust
let mut rows = Vec::new();
let mut items = subscription.timeout_iter_data(super::TAKE_FIRST_TIMEOUT);
loop {
    let waited = std::time::Instant::now();
    match items.next() {
        Some(Ok(Orders::OrderData(d))) => { /* filter-when-set + row push, unchanged */ }
        Some(Ok(_)) => {}   // OrderStatus variants skipped (unchanged posture)
        Some(Err(e)) => return Err(AppError::data(format!("completed orders stream: {e}"), "completed-orders")),
        None if waited.elapsed() >= super::TAKE_FIRST_TIMEOUT => {
            return Err(AppError::timeout(
                format!("no CompletedOrdersEnd within {}s — gateway did not answer reqCompletedOrders (known gateway issue; a restart may or may not cure it)",
                    super::TAKE_FIRST_TIMEOUT.as_secs()),
                "completed-orders",
            ))
        }
        None => break,   // instant None = stream self-ended on CompletedOrdersEnd => success
    }
}
```

Freeze coverage delta: none (spec-paths untouched — the seam/CLI contract don't cover the
drain). Review-by-reading gains: the timing-classified None arms + the per-item window.
