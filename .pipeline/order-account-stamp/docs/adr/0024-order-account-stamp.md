# ADR 0024 — orders carry the resolved account (choke-point stamping)

Status: accepted (2026-07-04, feature order-account-stamp)
Context: audit 2026-07-04 finding #2; closes the read/write account-authority split.

## Decision

1. **Stamp at the single placement choke point** (`place_with_client` gains a REQUIRED
   `&AccountId` parameter; body clones the order and sets `Order.account` via the pure
   seam `stamp_order_account`). REJECTED: per-verb stamping — a future verb could forget;
   review burden multiplies.
2. **Overwrite semantics**: the resolved account (cfg/`--account`, else first managed —
   `resolve_account`, the exact authority reads use) ALWAYS wins over any pre-set value.
   Deterministic; there is no legitimate second authority.
3. **Frozen builders untouched**: `build_stk_order`/`build_option_order`/
   `build_combo_order` signatures and output (`account: ""`) unchanged — the stamp is
   post-build, gateway-path; the three frozen order suites stay valid as-is.
4. **`option_close` passes its already-resolved account** — no duplicate
   `managed_accounts` round trip.
5. **Assumption + fallback**: the Tiger gateway accepts an explicitly-set `Order.account`
   (paper-probeable same-day: far-off ack, no fill needed). If it rejects, the fallback is
   "stamp only when `cfg.account` is explicitly set" — an OPERATOR decision recorded via
   journal observation, never auto-applied.

## Consequences

- `--account` now governs writes exactly as reads; the cross-account anti-open-gate defeat
  (close matched in A, order routed to default B) is closed.
- Multi-account sessions become safe-by-construction for every existing and future verb.
- One extra bounded `managed_accounts` read on stk/option/combo placements when no account
  is configured (option_close already paid it) — correctness over latency (ADR 0022/0023
  precedent).
