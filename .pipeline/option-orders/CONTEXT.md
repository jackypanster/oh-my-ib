# CONTEXT — option-orders

Delta on `.pipeline/stk-orders/CONTEXT.md` (write-path domain) + `.pipeline/options-read/
CONTEXT.md` (options domain). New/changed terms only:

- **Option order** — a single-leg option contract order: (symbol, expiry, strike, right)
  × (side, qty, limit). v1 is LMT/DAY only (ADR 0020): no MKT arm exists.
- **Marketable limit** — THE immediacy pattern: quote first (`option-quote`), then send a
  limit at/through the touch (buy ≥ ask, sell ≤ bid). Replaces MKT by design — bounded
  slippage on structurally wide books.
- **Whole-contract quantity** — options trade in integral contracts; `--qty` must be
  finite, ≥ 1, fract()==0. One contract controls `multiplier` (=100) shares.
- **Option ack** — exact 9 keys: `order_id, status, symbol, expiry, strike, right, action,
  quantity, limit_price` (limit always a number — LMT-only). The stk 6-key ack is a
  DIFFERENT frozen seam; they never merge.
- **Placement core** — the shared gate→connect→allocate→place→bounded-first-ack machine in
  `trade.rs` (`place_core`), used by stk and option verbs; ack shape injected per-verb.
- **Cancel reuse** — `omi cancel ORDER_ID` cancels ANY order (order-id domain is
  sec-type-agnostic); no option-cancel verb exists.

## Conventions (feature-specific)

- Validation ordering FROZEN: usage (local validation) < config (live gate) < connection.
- Every numeric CLI arg is finite-checked (NaN/inf ⇒ usage) — house rule since review-01.
- Write code ONLY in `trade.rs`; option identity parsing shared with the read path via
  pub(crate) `normalize_right`/`parse_expiry` (one parser, no drift).
- Acceptance is PAPER-only; paper options-permission is an environmental unknown —
  rejection = journaled observation, operator decision.
