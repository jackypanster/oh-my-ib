# ADR 0016 — Bound the completed-orders drain (live-proven wedge; amends ADR 0015)

Status: accepted (amends 0015's drain posture; evidence-first trigger FIRED)

## Context

ADR 0015 chose the unbounded `orders.rs` drain for `completed_orders(false)`, noting the
drain-wedge class was evidence-free. **The evidence arrived on this feature's own live
acceptance (2026-07-03, gateway build 2026-06-25, server v221):** `omi --live
completed-orders` hung >2.5min and again >45s on a HEALTHY gateway (health round-tripped
between runs; first hang was the session's FIRST completed-orders request — no kill
pollution). `CompletedOrdersEnd` never arrived. This matches a known upstream class:
ib_insync #224 (reqCompletedOrders hangs, workaround = don't call it) and twsapi groups.io
"cannot make working reqCompletedOrders" — some gateway builds/states never answer.

## Decision

1. **Bound the drain per item** with the existing shared const: replace bare
   `iter_data()` with an explicit loop over `timeout_iter_data(TAKE_FIRST_TIMEOUT)`.
2. **Classify the terminating `None` deterministically by wait duration**
   (`stream_ended` has no public accessor in ibapi-3.1.0):
   - wrap each `.next()` in `Instant::now()`; on `None`:
     - `elapsed >= TAKE_FIRST_TIMEOUT` ⇒ the wait starved ⇒
       `AppError::timeout("no CompletedOrdersEnd within 10s — gateway did not answer
       reqCompletedOrders (known gateway issue; a restart may or may not cure it)",
       "completed-orders")`, exit 6;
     - instant `None` (elapsed ≪ window) ⇒ the subscription self-ended on
       `CompletedOrdersEnd` ⇒ SUCCESS with whatever rows arrived.
   - `Some(Ok)/Some(Err)` arms unchanged.
3. **PRD criterion 8 amended** (recorded in PRD §Amendment): on gateways exhibiting the known
   issue, the bounded exit-6 timeout IS the acceptance PASS for the failure path; the
   exit-0 `{"completed_orders": []}` happy path is verified on any gateway/session that
   answers (fresh-session retry recommended; full row-content verification rides the first
   active trading day on a working gateway).

## Rationale

- Same defense class as ADR 0012 (read-timeouts), now extended to a drain with live
  evidence — exactly the boundary ADR 0012 drew ("a drain wedge is a different failure
  class and a future feature if evidence appears"). The evidence appeared.
- Per-item window (not whole-drain deadline): a large completed-orders day streams many rows;
  each arrival resets the window — bounded staleness without penalizing volume.
- Timing classification is sound: the timeout `None` cannot return before the window expires
  (deadline loop, sync.rs:226-236); the ended `None` returns in microseconds
  (`stream_ended` short-circuit, sync.rs:223-225). Filtered notices restart the window
  (ADR 0012's documented caveat) and correctly count as "traffic arrived".
- Rejected: exposing/patching ibapi for `stream_ended` (fork cost for one bit); whole-command
  wall-clock deadline (punishes big days); dropping the feature (loses the triad closer —
  the wedge may be session-state-dependent, as reqPnL's first-slot behavior proved).

## Consequences

- `omi completed-orders` can never hang: worst case 10s per silent wait, exit 6 with an
  actionable envelope; healthy gateways behave identically to ADR 0015's design.
- The known-issue wedge joins the gateway dossier (memory + review evidence): this gateway
  build now has TWO endpoint-level wedge behaviors (reqPnL first-slot-only; reqCompletedOrders
  no-answer). Fresh-session behavior to be observed at next restart.
- Frozen spec unchanged (the pure seam + CLI contract don't touch the drain) — no re-freeze;
  the fix is impl-paths-only on the open PR #14 branch.
