# ADR 0015 — `omi completed-orders`: drain-to-End, api_only=false, 14-key row

Status: accepted

## Context

The order-lifecycle reads were incomplete: `orders` (working orders, OpenOrderEnd drain) and
`executions` (fills + commissions, ExecutionDataEnd drain) exist; terminal order states
(filled/cancelled/rejected with status) had no read. TWS ships `reqCompletedOrders`;
ibapi-3.1.0 exposes it as `completed_orders(api_only) -> Subscription<Orders>` on a shared
channel whose response set `[CompletedOrder, CompletedOrdersEnd]`
(messages/shared_channel_configuration.rs:45) self-terminates the subscription iterator.

## Decision

1. `omi completed-orders` drains `completed_orders(false)` via `iter_data()` to natural End —
   the `orders.rs` pattern verbatim. `Orders::OrderData` arm only; `OrderStatus` variants
   skipped (sibling posture).
2. **`api_only = false`, hardcoded, no flag** — the operator trades via the Tiger app;
   an API-only view is empty by construction. YAGNI on the flag.
3. **Row = open-orders 10-key parity + 4 completion keys** (`status` Debug-rendered
   `OrderStatusKind`, `filled_quantity`, `completed_time`, `completed_status`), built through
   the pure frozen seam `shape_completed_orders` (plain `CompletedOrderRow` structs — the
   pnl-by-position/search freeze pattern). `average_fill_price` deliberately excluded: it
   lives on `Orders::OrderStatus`, not `OrderData`, and `executions` already carries fill
   prices.
4. **`--account` filter = `orders` SEMANTICS** (filter rows only when explicitly set; never
   auto-filter to the resolved account — ADR 0011 precedent). Implemented inline (no
   `_with_client` seam): completed-orders has no `brief` consumer — brief's 8-key top level
   is frozen spec (adding a section = re-freeze = separate feature).
5. Wrapper `{"completed_orders": [...]}` — the `orders`/`open_orders` convention.

## Rationale

- Drain-to-End is the correct class: End marker exists, stream self-terminates — NOT
  ADR 0012's markerless take-first class; wrapping the drain in a timeout would contradict
  ADR 0012's evidence-first boundary (no drain wedge ever observed).
- The pure-seam freeze keeps the gateway fn review-by-reading (no fake IB server, no-mock
  rule) while pinning the row contract offline — the house pattern since pnl-by-position.
- READ-ONLY red line: the diff contains request/drain/emit only — no place/modify/cancel
  paths (repo rule: trading is a later, gated phase).

## Consequences

- Day review completes: `orders` (working) + `executions` (fills) + `completed-orders`
  (terminal states incl. cancellations) — the agent joins by order_id/conid client-side.
- On a no-trade day the command returns `{"completed_orders": []}` — a valid PASS; row-content
  live verification rides the first active trading day (alongside the standing reqPnLSingle
  observation).
- If a future feature wants completed orders inside `brief`, that is a brief re-freeze
  decision (new spec-rev), not an edit here.
