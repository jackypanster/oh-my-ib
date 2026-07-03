# PRD — stk-orders (Phase 2 opener)

Feature: `omi buy` / `omi sell` / `omi cancel` — the first WRITE path: US-stock order placement
and cancellation, paper-first, live double-gated. **This feature deliberately amends the
repo's standing "read-only, no order-placement code" red line** — authorized by the operator
in this PRD's grilling (2026-07-03: "开启 Phase 2：STK 下单（paper 优先）").
Status: decision-complete (grilled 2026-07-03, operator locked D1–D5; ibapi write surface
verified present in crate source; exact call shapes are arch's to pin).

## Problem

`omi` can observe everything (13 read commands incl. the order-lifecycle triad) but act on
nothing. The agent's end-goal — execute the daily decisions it derives from `brief`/`quote` —
requires a write path. Trading was always the project's "later, gated phase"; the operator has
now opened it, STK-only, paper-first.

## Goal

Three verbs on the existing connect-per-command model:
- `omi buy SYM QTY [--limit PRICE]` — BUY LMT (with `--limit`) or MKT (without), TIF=DAY.
- `omi sell SYM QTY [--limit PRICE]` — mirror.
- `omi cancel ORDER_ID` — cancel a working order.
Paper (`:4002`, the default port) needs no extra gate; live placement requires BOTH `--live`
AND `OMI_ALLOW_LIVE=1`. Every verb returns a bounded, deterministic ack.

## Success criteria (acceptance)

1. `omi buy AAPL 1 --limit <far-below-market>` (paper default) places the order and exits 0
   within ~10s printing an ack object: `{order_id, status, symbol, action, quantity,
   limit_price}` (`limit_price` null for MKT; `status` = the first order-state ack from the
   gateway, e.g. PreSubmitted/Submitted).
2. `omi sell` mirrors (action SELL). No `--limit` ⇒ MKT order; with ⇒ LMT. TIF=DAY hardcoded.
3. `omi cancel <order_id>` exits 0 with a bounded ack (status e.g. Cancelled/PendingCancel).
4. **Live double gate (frozen, offline-testable):** `--live` WITHOUT `OMI_ALLOW_LIVE=1` ⇒
   `code="config"` envelope BEFORE any connection attempt, non-zero exit, for all three verbs.
   With both present the command proceeds (dead-port test ⇒ `code="connection"`).
5. Paper writes need no env gate (the sandbox is for using); reads keep their existing gates
   (unchanged everywhere).
6. **Ack timeout** (gateway accepts but no order event within the shared 10s bound) ⇒ exit 6
   `code="timeout"`, message explicitly: order MAY have been submitted — verify with
   `omi orders`; do NOT blindly retry (double-placement risk). No automatic retry anywhere.
7. Local validation (usage errors, offline-frozen): qty must be > 0; `--limit` must be > 0
   when present; missing args ⇒ usage envelope.
8. READ commands byte-identical (no shared-code regression); the whole existing frozen suite
   stays green untouched.
9. **Docs amendment rides the PR**: AGENTS.md + CLAUDE.md red line updated from "no
   order-placement code" to the Phase-2 truth (writes exist; paper default; live double-gated;
   reads unaffected). Operator-authorized by this PRD.
10. `cargo build` · clippy `-D warnings` · `cargo test` green.
11. **Merge gate (operator, live-on-PAPER):** full lifecycle on a PAPER gateway session
    (`:4002`): place far-from-market LMT buy → `omi orders` shows it working → `omi cancel` →
    `omi completed-orders` shows Cancelled → no position change (`omi positions`).
    PREREQUISITE: the operator logs the gateway into the PAPER account for this acceptance
    (IB Gateway is live XOR paper per session; all prior acceptance ran live).

## Scope

- `src/cli.rs`: `Buy(OrderArgs)`, `Sell(OrderArgs)`, `Cancel(CancelArgs)` variants.
- `src/ib/place.rs` (new; exact module split arch's call): pure order-building seam
  (side/qty/limit → order fields: LMT vs MKT, TIF DAY, action) + pure ack-shaping seam +
  gateway fns (place with bounded first-event wait; cancel with bounded ack).
- Gate check helper (config-level, pre-connect): live-write ⇒ require `OMI_ALLOW_LIVE=1`.
- `src/ib/mod.rs` + `src/main.rs` wiring. AGENTS.md/CLAUDE.md §red-line amendment.
- No new dependency.

## Non-scope (explicitly NOT this feature — v1 boundaries)

- **No modify/replace** (operator D5): agent uses cancel + re-place; IB modify semantics are
  a minefield deferred to v2.
- **No notional cap** and **no `--dry-run`/whatIf preview** — offered in grilling, operator
  explicitly did NOT select them. Recorded as deliberate v1 omissions (v2 candidates).
- No GTC / outside-RTH / stop / stop-limit / trailing / bracket / OCA — LMT + MKT, DAY only.
- No options, no combos, no non-STK sec-types (STK guard like quote's).
- No auto-retry, no order-status polling loop, no `--watch` — one bounded ack per invocation.
- No fractional-share validation beyond qty > 0 (broker rejects what it rejects).

## Resolved decisions (locked)

- D1 **Phase 2 opened, STK orders first** (operator, grilled): over options-data-read and
  fx-quote. The red-line amendment is part of the feature, not a side effect.
- D2 **Verbs `buy`/`sell`/`cancel` + LMT/MKT, TIF=DAY** (operator, preview-confirmed):
  verb-per-intent minimizes agent misuse; unified `order place --side …` rejected (verbose,
  larger typo surface); LMT-only rejected (MKT wanted, slippage is the agent's judgment).
- D3 **Safety = paper-free + live double-gate ONLY** (operator, multi-select: baseline only):
  live writes need `--live` ∧ `OMI_ALLOW_LIVE=1` (missing either ⇒ config error pre-connect);
  paper writes ungated beyond the default-port model. Notional cap and whatIf preview
  deliberately not selected.
- D4 **Bounded first-ack semantics** (operator): reuse the shared 10s const; timeout ⇒
  UNKNOWN-state envelope naming the verification command and forbidding blind retry.
- D5 **v1 = place + cancel** (operator): modify deferred.

## Risks / fragile assumptions

- **Highest-stakes feature so far**: a bug places real orders. Mitigations: paper default;
  live double-gate frozen-tested offline; acceptance entirely on paper; the review READ-ONLY
  grep flips polarity — review must verify writes exist ONLY in the new module and are
  unreachable from every read command path.
- ibapi write-call shapes (place_order/submit_order/order_update_stream/next_valid_order_id,
  builder `analyze`) verified PRESENT but their exact sync semantics (what the first event
  stream yields, order-id allocation, cancel ack channel) are arch's to pin from source —
  the ack design may shift shape (e.g. which event constitutes "first ack") without changing
  the criteria above.
- Paper-account entitlements/behavior may differ from live (fills, market data on paper) —
  acceptance uses far-from-market LMT + cancel to avoid fill dependence.
- The known gateway wedge dossier (reqPnL first-slot, reqCompletedOrders intermittent) may
  extend to order channels — every wait is bounded by design (D4), so worst case is a clean
  timeout envelope, never a hang.
- Rollback: additive module + CLI verbs; revert removes the write path entirely; the docs
  amendment reverts with it.

## Verification

- Offline frozen spec: the pure order-building + ack-shaping seams; the double-gate config
  errors (all three verbs × missing-gate matrix — fully offline, gate precedes connect);
  arg-validation usage errors; dead-port connection envelopes (gates satisfied);
  help lists the three verbs; existing suite untouched.
- Live (operator, PAPER): criterion 11 lifecycle.
