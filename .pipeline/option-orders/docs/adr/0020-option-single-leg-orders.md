# ADR 0020 — Single-leg option orders: LMT-only, whole contracts, shared placement core

Status: accepted (option-orders; Phase 2 step 3 — extends the ADR 0017/0018 write path,
adds NO new safety machinery)

## Context

The write path (stk buy/sell/cancel, ADR 0017) and the options read path (chain/quote,
ADR 0019) exist. Single-leg option placement needs three decisions with real alternatives:
order-type surface, quantity semantics, and how the new path shares the proven placement
machinery. The operator granted full-auto authority for this feature; D1 (LMT-only) is the
orchestrator's judgment call, explicitly flagged for operator override at the merge gate.

## Decision

1. **LMT-only v1** (`--limit` required; no MKT arm exists in `build_option_order`).
   Option books are structurally wider and thinner than equities; a MKT order on an
   illiquid strike fills at the far touch — the classic retail options footgun. A
   *marketable* limit (buy at/above ask) achieves immediate execution with bounded
   slippage, so nothing is lost operationally. MKT is additive later if wanted.
   (Deviation from stk-orders D2 where the operator chose MKT+LMT for stocks — deliberate:
   the liquidity structure differs and the standing authority covers it.)
2. **Whole-contract quantity**: `qty` must be finite, ≥ 1, and integral (`fract() == 0`);
   fractional contracts do not exist. Every numeric arg (qty/strike/limit) is
   finite-checked — the review-01 NaN/inf lesson applied at design time, not caught at
   review time.
3. **Shared placement core**: `place()`'s gate→connect→allocate→place→bounded-first-ack
   steps are extracted into a contract-agnostic `place_core` used by both stk and option
   verbs, with ack shaping injected as a closure so the two frozen ack seams (6-key stk,
   9-key option) stay pure and disjoint. The frozen stk suite is the refactor's
   regression net.
4. **Cancel is NOT duplicated**: `cancel_order` is order-id-based; the existing `omi
   cancel` verb + its frozen tests already cover option orders.
5. **9-key option ack** `{order_id, status, symbol, expiry, strike, right, action,
   quantity, limit_price}` — an option ack without contract identity would be ambiguous
   the moment two strikes are working; `limit_price` is always a number (LMT-only).

## Rationale

- One placement machine, two thin verbs: every future sec-type write reuses `place_core`;
  divergence bugs (fix stk, forget options) become structurally impossible.
- LMT-only shrinks the blast radius of the highest-risk surface this repo has: the worst
  agent mistake becomes "an unfilled resting order", not "a fill at any price".
- Validation reuse (`normalize_right`/`parse_expiry` promoted to `pub(crate)`) keeps ONE
  parser for option identity across read and write paths — quote and order can never
  disagree about what "20260918 C 250" means.

## Consequences

- An agent wanting immediacy quotes first (`option-quote`), then sends a marketable limit —
  documented in CONTEXT.md as THE pattern.
- Adding MKT later = one optional flag + one arm in the builder + spec append-card; nothing
  here blocks it.
- The place-core extraction slightly reshapes `trade.rs` internals under the stk feature's
  frozen tests — accepted: those tests assert the observable contract, which is the point.
