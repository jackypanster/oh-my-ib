# PRD — completed-orders

Feature: `omi completed-orders` — today's completed (filled / cancelled / rejected) orders with
status, closing the order-lifecycle read triad: `orders` (working) + `executions` (fills) +
`completed-orders` (terminal states incl. cancellations).
Status: decision-complete (grilled 2026-07-03, operator locked D1–D2; code facts verified in
ibapi crate source and src/ib/orders.rs, not guessed).

## Problem

The day-review flow ("今天的单子都怎么样了") has a hole: `omi orders` shows only WORKING
orders, `omi executions` only FILLS. Cancelled / rejected / fully-filled orders with their
terminal status are invisible — exactly the rows a post-trade review needs. TWS ships
`reqCompletedOrders`; unused so far. Value activates fully at the next phase (active trading),
cost is minimal now: the drain pattern is `orders.rs` verbatim.

## Goal

New read-only subcommand `omi completed-orders`: connect, ONE `completed_orders(false)` drain
to `CompletedOrdersEnd`, emit `{"completed_orders": [rows…]}` mirroring the `orders` command's
shape conventions, disconnect.

## Success criteria (acceptance)

1. `omi completed-orders` (paper default) exits 0 and prints ONE JSON object
   `{"completed_orders": [...]}` (the `orders`/`open_orders` wrapper convention).
2. Each row carries the open-orders identity keys (order_id, account, symbol, conid, action,
   quantity, order_type, limit_price, aux_price, tif — exact 10-key parity where fields exist)
   PLUS completion fields (status, completed_time, completed_status; exact additional set is
   arch's to pin from ibapi `OrderData`/`OrderState` source). Money/optional fields follow
   house sentinel rules.
3. No completed orders today ⇒ `{"completed_orders": []}`, exit 0.
4. `--account` filter semantics IDENTICAL to `omi orders` (ADR 0011): filter rows ONLY when
   `--account` is explicitly set; never auto-filter to the resolved account.
5. READ-ONLY red line: request-only (`completed_orders`, drain, emit) — no order placement /
   modification / cancellation code paths anywhere in the diff.
6. Gateway down ⇒ existing connection-error contract; `--help` lists `completed-orders`.
7. `cargo build` · `cargo clippy --all-targets -- -D warnings` · `cargo test` green; all
   existing frozen specs untouched.
8. **Merge gate (operator, live):** `omi --live completed-orders` exits 0 with the correct
   shape; on a no-trade day `[]` is the expected PASS (content is gateway/day-dependent; shape
   + exit code are the acceptance).

## Scope

- `src/cli.rs`: `CompletedOrders` variant (house hyphenation precedent: `pnl-by-position`).
- `src/ib/completed_orders.rs` (new): drain fn mirroring `orders.rs` — the row loop over
  `Orders::OrderData`, filter-when-set, End-terminated `iter_data()` (the subscription ends
  itself on `CompletedOrdersEnd` — shared-channel response set, verified:
  messages/shared_channel_configuration.rs:45).
- `src/ib/mod.rs` + `src/main.rs`: wiring. No new dependency.

## Non-scope (explicitly NOT this feature)

- No new `brief` section — brief's 8-key top level is FROZEN spec; extending it is a separate
  feature with a re-freeze decision.
- No `api_only=true` mode/flag (D4): the operator trades via the Tiger app (non-API), so
  api_only would show nothing by design; hardcode `false`, no flag (YAGNI).
- No date-range / status-filter flags — pass through what the gateway returns for the day.
- No pagination, no sorting beyond gateway order.

## Resolved decisions (locked)

- D1 **Feature choice = completed-orders** (operator, grilled 2026-07-03). Picked over
  FX/CASH quote (medium complexity, value deferred until multi-currency positions exist) and
  account_summary-tags (marginal). Closes the order-lifecycle triad at near-zero risk.
- D2 **Team rotation: keep last round's assignment** (operator): π/omp (GLM-5.2) = impl,
  codex cli (GPT-5.5) = review, Claude Code (Fable 5) = prd/arch/task/orchestration/merge.
  (Paradigm: roles rotate freely, every stage SOTA — KB note 41.100.)
- D3 **Name = `completed-orders`** (code/house style): hyphenated multi-word precedent
  `pnl-by-position`; unambiguous next to `orders`.
- D4 **`api_only = false` hardcoded** (code): include ALL completed orders (app/manual +
  API); an API-only view is useless for this account today.
- D5 **Filter semantics = `orders` verbatim** (code, ADR 0011 precedent): `--account` set ⇒
  filter rows; unset ⇒ pass-through. Documented so review can byte-check against `orders.rs`.
- D6 **Drain shape = `orders.rs` verbatim** (code): `for item in subscription.iter_data()`,
  `Orders::OrderData` arm only, natural termination on the End message (stream self-ends; no
  markerless hazard, NOT ADR 0012's class — no timeout wrapping).

## Risks / fragile assumptions

- The exact completion-field set on `OrderData.order_state` (status / completed_time /
  completed_status availability) is server-version dependent — arch pins the emitted keys
  from ibapi struct+decoder source; absent → house null rules. Live acceptance confirms on
  the Tiger gateway.
- `Orders::OrderStatus` variants in the completed stream (if any) are skipped by the
  OrderData-only arm — same posture as `orders.rs` (verified there live since phase 1).
- A no-trade live day proves shape/exit only (criterion 8) — row-content verification rides
  the first active trading day alongside the standing reqPnLSingle observation.
- Rollback: purely additive subcommand.

## Verification

- Offline: frozen spec — pure row-shaping seam (arch to define, mirroring shape_* pattern) +
  CLI contract (help/dead-port/zero-state); card-scoped runner.
- Live (operator): criterion 8.

## Amendment (2026-07-03) — criterion 8 wording (evidence: live wedge, ADR 0016)

Live acceptance exposed a known gateway issue: reqCompletedOrders may never answer
(CompletedOrdersEnd absent ⇒ pre-fix hang; 2× local repro + upstream reports, ADR 0016).
Criterion 8 now reads: **PASS = either** (a) exit 0 with the `{"completed_orders": [...]}`
shape (gateway answered — `[]` fine on a no-trade day), **or** (b) the bounded exit-6
`timeout` envelope naming the known issue, delivered in ~10s (gateway exhibiting the issue —
the bound working IS the acceptance of the failure path). Fresh-session retry recommended at
the next gateway restart; row-content verification rides the first active trading day on an
answering gateway.
