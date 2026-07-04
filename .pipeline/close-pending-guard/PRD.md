# PRD — close-pending-guard (audit finding #1 fix)

Feature: make `omi option-close` refuse when a working close order already covers the conid —
the anti-double gate becomes a REAL gate against the double-fire path (retry after an UNKNOWN
timeout envelope). Brute-force v1, fail-closed, ONE card. No new safety machinery elsewhere;
ADR 0017/0018/0022 untouched.
Status: decision-complete (operator /think audit 2026-07-04 + explicit "开干"; full-auto).

## Problem

`option_close` derives side/qty from the POSITION SNAPSHOT only (ADR 0022 §2). It is blind to
in-flight orders: after a placement timeout (exit 6, "may have been SUBMITTED"), a retried
`option-close` sees the same position, places a SECOND full-size close, and when both fill the
position FLIPS through zero into the opposite side — the exact failure class this verb exists
to prevent. Today's only defense is envelope text ("do NOT retry blindly"). Audit 2026-07-04
ranked this the #1 live defect.

## Goal

Insert a pending-close guard into `option_close`: after the position match (and non-OPT
check), before `derive_close`, scan open orders ON THE SAME CLIENT; if ANY working order on
the same conid has action OPPOSITE to the position sign, refuse — naming every blocking
order id and pointing at `omi cancel <id>` / `omi orders`. Effect: retry-after-timeout
becomes SAFE (first order reached the gateway ⇒ scanned and refused; never reached ⇒ retry
proceeds legitimately). Same-side working orders (adds) never block.

## Success criteria (acceptance)

1. Pure seam (FROZEN, offline — `tests/close_pending_guard.rs`):
   `blocking_close_order_ids(position, conid, &orders)` matrix — long+working-SELL(same conid)
   ⇒ blocks; short+working-BUY ⇒ blocks; same-side ⇒ empty; other-conid ⇒ empty; multiple
   blockers ⇒ ALL ids, ascending; zero position handled (guard unreachable — position==0
   already refused upstream — seam still total: returns opposite-side matches).
2. Gateway ordering (review-by-reading): guard runs AFTER position match + non-OPT check,
   BEFORE derive/rebuild/assert/place; drain via the EXISTING shared
   `open_orders_with_client` on the SAME client (single-connect invariant, ADR 0022 §5 —
   zero new connects, zero new drain code).
3. Refusal envelope: `not_found` family, message contains conid, EVERY blocking order id,
   and both `omi cancel` and `omi orders` pointers.
4. Existing frozen suites BYTE-UNTOUCHED and green: `tests/option_close_command.rs`,
   `tests/positions_row.rs` (`derive_close` signature/behavior unchanged; guard refuses
   BEFORE derive_close runs).
5. `cargo build` · clippy `-D warnings` · full `cargo test` green.
6. AGENTS.md option-close phrase gains "refuses while a working close order exists".
   CLAUDE.md UNTOUCHED (no byte-budget interaction — verb list unchanged).
7. Merge gate (paper, DEFERRED with option-close criterion 12 to the next US trading
   session — one combined lifecycle): open option position → far-off `option-close`
   (working) → second `option-close` ⇒ REFUSED naming the working id → `omi cancel` →
   `option-close` marketable → flat. Offline gates + review are the merge basis today
   (same waiver pattern the operator granted for option-close).

## Scope

- `src/ib/trade.rs`: pure FROZEN seam `blocking_close_order_ids` + guard wiring in
  `option_close` (drain rows → extract `(order_id, conid, action)` triples → seam →
  refuse if non-empty).
- `tests/close_pending_guard.rs` (NEW spec file; the only frozen surface of this feature).
- AGENTS.md one-phrase amendment. Nothing else.

## Non-scope

- No qty-budget version (closable = |position| − pending): partial-fill semantics + more
  code; brute-force refuse is strictly safer and covers the real trigger. v2 candidate.
- buy/sell/option-buy/option-sell/option-combo remain open-tools (no position/order scan).
- Order.account fix (audit #2) — separate future feature.
- No new error code (`blocked`): reuse `not_found` to avoid touching shared error machinery.

## Resolved decisions (locked)

- D1 **Reuse `open_orders_with_client(client, None, ctx)`** (`orders.rs`, pub(crate),
  live-verified read path) — takes the already-connected client; zero new gateway surface.
- D2 **Blocking predicate = same conid ∧ opposite action ∧ present in `all_open_orders`**.
  No status filtering: `all_open_orders` returns only currently-open orders by definition
  (terminal orders live in completed-orders); a PendingCancel order still blocks =
  fail-closed correct.
- D3 **No account filter in the guard scan** (`account_filter=None`): a same-conid
  opposite-side order in ANOTHER account falsely blocks — fail-closed, acceptable;
  single-account today; account-awareness belongs to audit #2.
- D4 **Refusal = `not_found` family** with ids + remediation pointers (D3 of option-close
  precedent: "not in a closable state"); new error code rejected (non-scope).
- D5 **Seam input = plain triples `(order_id: i32, conid: i32, action: &str)`** extracted
  from the shared drain's rows ("Buy"/"Sell" Debug strings, orders.rs:38 precedent);
  output = ascending Vec<i32> (deterministic, freeze-able).

## Risks / fragile assumptions

- **`all_open_orders` completeness on Tiger**: assumed to return every working order for
  the session's account(s). Live-verified for API-placed orders (order 3, 2026-07-04
  acceptance). If it ever under-reports, the guard silently passes — residual risk equals
  today's status quo (never worse). Deferred paper lifecycle (criterion 7) observes.
- **TOCTOU residual (accepted, documented)**: scan→place is not atomic; two strictly
  concurrent closes could pass each other. Fixed client_id makes the second concurrent
  connect FAIL (de facto mutex); the realistic sequential trigger is fully killed.
- Rollback: one revert removes the guard; no schema/CLI change.

## Verification

- Offline frozen: seam matrix (criterion 1) — pure fn, no gateway.
- Review-by-reading: guard placement/ordering, single-connect, refusal message content,
  spec-paths untouched (`option_close_command.rs`, `positions_row.rs`).
- Live: deferred combined lifecycle (criterion 7).
