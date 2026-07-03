# ADR 0017 — The write path: double gate, bounded ack, no retry, paper-first

Status: accepted (Phase 2 opener; amends the repo's founding read-only red line —
operator-authorized in the stk-orders PRD grilling, 2026-07-03)

## Context

Phase 1 froze a hard rule: no order-placement code. The operator opened Phase 2 (STK
buy/sell/cancel). A write path in an agent-driven CLI is the highest-risk surface this repo
will ever add: a bug or a confused agent places real orders with real money.

## Decision — the safety architecture

1. **Containment**: ALL write calls (`place_order`/`cancel_order`/encoders) live in ONE
   module, `src/ib/trade.rs`. Review enforces by grep: write symbols appear nowhere else;
   no read command imports the module. The red-line docs are amended (verbatim text in
   arch.md) rather than silently contradicted.
2. **Double gate on live, ungated paper**: an order at the EFFECTIVE live port
   (`cfg.port == LIVE_PORT`, catching both `--live` and hand-set `--port 4001`) requires
   `OMI_ALLOW_LIVE=1` in the environment; missing ⇒ `code="config"` BEFORE any connection
   (offline-deterministic ⇒ frozen-testable). Paper (`:4002`, the default port) is the
   sandbox — no extra gate. Reads keep their existing (gate-free) behavior everywhere.
3. **Bounded, deterministic ack**: allocate the order id FIRST (`next_valid_order_id`), then
   `place_order` and wait for the FIRST `OrderStatus`/`OpenOrder` event under
   `TAKE_FIRST_TIMEOUT` (per-item window, ADR 0016's Instant-classified pattern; skip
   ExecutionData/CommissionReport). Success ⇒ a 6-key ack echoing the request + gateway
   status. No event ⇒ exit 6 `timeout` envelope that NAMES the allocated order id, says the
   order MAY be submitted, points at `omi orders`, and forbids blind retry.
4. **No retry, ever**: a placement timeout is an UNKNOWN state, not a failure to redo —
   automatic re-placement is the classic double-order bug. The agent verifies via the read
   triad (`orders`/`executions`/`completed-orders`) that already exists for exactly this.
5. **v1 surface** (operator-locked): LMT/MKT, TIF=DAY, STK only; place + cancel; no modify,
   no notional cap, no whatIf dry-run (explicitly deselected), no GTC/stops/brackets.
6. **Acceptance on PAPER only** (criterion 11): full lifecycle with a far-from-market LMT
   (place → visible working → cancel → Cancelled → positions unchanged). Live trading is
   never exercised by the pipeline; the double gate is the operator's own key.

## Rationale

- Gate-on-effective-port beats gate-on-flag: `--port 4001` without `--live` must not bypass.
- Env-var over config-key: a config file is durable state an agent might edit; an env var is
  per-invocation, visible in the process record, and never persists by accident.
- First-ack-of-either-kind (OrderStatus OR OpenOrder) absorbs gateway ordering variance
  without a second wait.
- Order-id-before-place means even the worst outcome (timeout) yields an actionable handle.
- Reuse of `TAKE_FIRST_TIMEOUT` + the ADR 0016 wait pattern keeps ONE timeout vocabulary
  across the repo (no new constants, no new error codes).

## Consequences

- The repo is no longer read-only; the founding docs say so explicitly (amended paragraph).
- The agent gains act-capability with the same JSON/exit-code contract as reads; its
  workflow becomes brief → decide → buy/sell → verify via reads.
- Every future write feature (modify, options, combos) inherits this architecture:
  containment module, double gate, bounded ack, no retry, paper-first acceptance.
- If the order channel ever wedges (gateway dossier has two read-side precedents), the
  bounded ack turns it into a 10s UNKNOWN-state envelope — never a hang, never a duplicate.
