# CONTEXT — close-pending-guard

Delta on `.pipeline/option-close/CONTEXT.md`. New/changed terms:

- **Pending-close guard** — the option_close step between position match and derivation:
  scan working orders on the SAME client; same-conid + opposite-side ⇒ refuse naming ids.
  Completes the anti-double gate (ADR 0022 §2) against the double-fire path.
- **Double-fire** — two close orders for the same position both filling ⇒ the position
  flips through zero. Classic trigger: retry after an exit-6 UNKNOWN timeout envelope.
- **Blocking order** — a working order with the SAME conid and action OPPOSITE to the held
  position's sign (long ⇒ working Sell; short ⇒ working Buy). Same-side working orders
  (adds) never block. Any-account, any-status-in-open-orders (both fail-closed, ADR 0023).
- **Safe retry** — post-guard property: if the first close reached the gateway, the retry
  is refused (guard sees it); if it never reached, the retry proceeds — the UNKNOWN state
  resolves itself mechanically.

## Conventions (feature-specific)

- Refusal ordering extends the frozen chain: … < position-match(not_found) < non-OPT(usage)
  < pending-guard(not_found) < derive < rebuild < conid-assert < place.
- Drain reuse: `open_orders_with_client` VERBATIM (orders.rs, pub(crate), brief precedent);
  the guard adds no connects and no new gateway calls.
- Malformed drain row ⇒ `data` error naming the row index — never skip (ADR 0023 §5).
- CLAUDE.md untouched this feature; AGENTS.md gains one phrase.
