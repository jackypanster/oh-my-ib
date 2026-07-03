# CONTEXT — completed-orders

New domain terms. Base glossary: `.pipeline/phase1-readonly/CONTEXT.md`; reuses
**drain-to-End** (executions-command), **filter-when-set** (ADR 0011). Only deltas here.

## Completed-orders domain

- **Order-lifecycle triad** — `orders` (working, OpenOrderEnd) + `executions` (fills,
  ExecutionDataEnd) + `completed-orders` (terminal states, CompletedOrdersEnd). The agent
  joins by `order_id`/`conid` client-side.
- **Terminal state** — an order the gateway reports as done for the day: Filled / Cancelled /
  rejected etc.; rendered in the row as `status` (Debug-rendered `OrderStatusKind`) +
  `completed_status`/`completed_time` (gateway strings, "" when omitted).
- **`CompletedOrderRow`** — plain 14-field struct (open-orders 10-key parity + 4 completion
  keys); frozen test constructs these directly.
- **`shape_completed_orders`** — the pure FROZEN seam: rows in gateway order → JSON array of
  exact 14-key objects; empty ⇒ `[]`.
- **`api_only=false`** — include app/manual orders (the operator trades via the Tiger app);
  hardcoded, no flag (ADR 0015).

## Conventions (feature-specific)

- Wrapper: `{"completed_orders": [...]}` (the `orders`/`open_orders` convention).
- `--account` filter: rows filtered ONLY when `--account` explicitly set (ADR 0011 semantics,
  `orders` parity); implemented inline — no `_with_client` seam (no brief consumer; brief's
  top level is frozen).
- `limit_price`/`aux_price`: `Option<f64>` raw pass-through (None → null) — orders.rs parity;
  NOT `pnl_number` (no IBKR sentinel on these).
- READ-ONLY red line: request/drain/emit only; no place/modify/cancel code anywhere.
- Drain class: End-marked, self-terminating — no TAKE_FIRST_TIMEOUT (ADR 0012 boundary).
- **Merge gate (PRD criterion 8)**: `omi --live completed-orders` shape/exit PASS on any day;
  `[]` on a no-trade day is a PASS.
