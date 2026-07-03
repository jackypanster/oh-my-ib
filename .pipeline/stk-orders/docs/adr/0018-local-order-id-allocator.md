# ADR 0018 — Allocate order ids from the handshake-seeded local counter (amends ADR 0017)

Status: accepted (paper-acceptance wedge routing; amends 0017's order-id step)

## Context

PAPER acceptance of the first `omi buy` HUNG >3min before any order reached the gateway
(verified: `orders` and `completed-orders` both empty after the kill — the hang preceded
`place_order`). Root cause pinned in ibapi-3.1.0 source: ADR 0017 chose
`next_valid_order_id()`, which is an UNBOUNDED server round-trip —
`subscription.next()` with no timeout (orders/sync.rs:197) on a RequestIds shared request
this gateway apparently never answers. Third endpoint-wedge for the gateway dossier
(after reqPnL first-slot and reqCompletedOrders intermittent), first observed on PAPER.
ADR 0017's "bounded request-response, crate-managed" claim about this call was WRONG.

## Decision

Use the crate's **local, non-blocking, handshake-seeded allocator** instead:
`Client::connect` populates `ClientIdManager::new(connection_metadata.next_order_id)` from
the connect handshake's NextValidId (client/sync.rs:121, connection.rs:14);
`client.next_order_id()` just increments that counter (id_generator.rs:86-88). Swap
`client.next_valid_order_id()?` → `client.next_order_id()` in `trade.rs`. No other change.

## Rationale

- The handshake ALWAYS delivers NextValidId (TWS protocol: sent right after StartApi) — the
  extra server round-trip bought nothing and introduced an unbounded wait our architecture
  forbids.
- Connect-per-command means one order per connection: the handshake-seeded id is exactly the
  server's declared next id — no staleness window.
- Alternative rejected: wrapping `next_valid_order_id` in our own bounded subscription —
  impossible without forking (the blocking `next()` is inside the crate method).

## Consequences

- No unbounded wait remains anywhere on the write path (gate → connect → local id →
  place_order → bounded ack).
- The UNKNOWN-state timeout envelope still names the allocated id (allocation is now
  infallible-local).
- Gateway dossier: RequestIds joins the no-answer list; any future feature needing an
  explicit id refresh must bound it itself.
