# CONTEXT — read-timeouts

New domain terms for this feature. Base glossary: `.pipeline/phase1-readonly/CONTEXT.md`; reuses
**take-first** / **unset sentinel** (pnl-command), **discovery** / **sweep** (pnl-by-position),
**sequential fetch discipline** (brief-command). Only the deltas live here.

## Timeout domain

- **Wedge** — the live-proven gateway-state hazard (2026-07-03, build 2026-06-25): the gateway
  accepts a `reqPnL`/`reqPnLSingle` subscription but never emits a tick; killed client processes
  orphan their subscriptions and pollute the PnL channel for ALL clients until a gateway restart.
  Evidence: `.pipeline/brief-command/reviews/review-01.md` §Live acceptance.
- **Timeout twin** — `timeout_iter_data(d).next()`, the crate's data-only, notice-filtered,
  bounded equivalent of `next_data()` (≡ `iter_data().next()`). ADR 0012's mechanism.
- **`TAKE_FIRST_TIMEOUT`** — the ONE shared pub const (10s, `src/ib/mod.rs`) bounding both
  take-first seams. Fixed, not configurable (PRD D3).
- **None-collapse** — after the swap, a `None` read means "timed out" OR "stream ended before
  data" (instant); both map to the `timeout` error. The old `data`/"no PnL reading" arm is gone
  (ADR 0012 Consequences).
- **Cure message** — the timeout error's message names the wait and the operator action:
  `… within 10s — gateway PnL channel may be wedged; restart the gateway`. The sweep arm
  prefixes the offending conid (fail-fast attribution, ADR 0009 discipline).

## Conventions (feature-specific)

- Error envelope shape unchanged: `{"error":{code,message,context}}`; new code `"timeout"` ⇔
  exit 6 (existing: connection=2, not_found=3, data=4, config=5, usage=64, error=1).
- Healthy-path stdout is byte-identical on all four call paths (`pnl`, `pnl-by-position`, both
  brief sections) — this feature changes ONLY the no-data failure path (PRD criterion 6).
- Scope boundary: take-first reads only. Drain-to-End loops (`account_updates`, orders,
  executions), `quote` (SnapshotEnd), request-response calls, and the connect phase keep their
  posture (PRD non-scope; ADR 0012 licenses no blanket timeouts).
- brief stays whole-command fail-fast (brief PRD D3): a timeout in any step is the command's
  single structured error.
- **Merge gate (PRD criterion 8)**: operator live-accepts `omi --live pnl` + `omi --live brief`
  healthy-path speed (seconds, never 10s) BEFORE the PR merges; the timeout path itself is
  review-by-reading (no fake IB server — no-mock rule).
