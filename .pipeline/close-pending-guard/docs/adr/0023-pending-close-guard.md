# ADR 0023 — pending-close guard: refuse while a working close order exists

Status: accepted (2026-07-04, feature close-pending-guard)
Context: audit 2026-07-04 finding #1; completes the ADR 0022 anti-double gate.

## Decision

1. **Brute-force refuse, not qty-budget**: ANY working same-conid opposite-side order
   blocks the whole close. REJECTED: closable = |position| − pending (needs partial-fill
   remaining-vs-total semantics for marginal value; the real trigger — retry after an
   UNKNOWN timeout — is fully covered by refuse).
2. **No status filtering**: the shared drain returns only `Orders::OrderData` from
   `all_open_orders()` — open orders by definition. A PendingCancel order still blocks
   until the gateway confirms the cancel (fail-closed).
3. **No account filtering** (`account_filter=None`): a same-conid opposite order in another
   account falsely blocks — fail-closed and irrelevant single-account; account-awareness
   is audit finding #2's scope (Order.account), not this guard's.
4. **`not_found` reuse** for the refusal ("not in a closable state" family, option-close
   precedent). REJECTED: a new `blocked` error code — touches shared error machinery for
   one message; additive code churn without agent-facing gain.
5. **Fail-closed row parsing**: a drain row missing order_id/conid/action ⇒ `data` error
   naming the row — NEVER skip-and-continue (a skipped row could hide the blocker that
   the guard exists to see).
6. **TOCTOU residual accepted**: scan→place is not atomic. Two strictly concurrent closes
   could pass each other's scan; the fixed client_id rejects the second concurrent connect
   (de facto mutex), and the sequential trigger is eliminated. Revisit only if client_id
   ever becomes per-invocation.

## Consequences

- Retry-after-timeout becomes mechanically safe; the envelope's "do NOT retry blindly"
  is now enforced, not advised.
- Stacked partial closes require an explicit `omi cancel` first — accepted UX cost.
- The guard adds one bounded read (~`omi orders` cost) per close — correctness over
  latency on the write path (ADR 0022 consequence extended).
