# PRD — reprice

Stage: prd · feature: reprice · repo: jackypanster/oh-my-ib · branch: cursor/reprice-open-order-e22d
Operator-approved (2026-10-22): close the first gap vs the sister Futu CLI `ftnn`.
Amends ADR 0017 §5 ("no modify") and the placement-timeout exit (was 6; writes are now 8).

## Problem

`omi` can place and cancel but cannot change the limit of an order that is already working.
The agent cancels and replaces, which loses queue position and can double-fire if the cancel
outcome is unknown. `ftnn` already has modify-in-place plus a caller-stable exit-code contract
(4 / 7 / 8 / 64) that `omi` writes do not match.

## Goal

1. `omi reprice --price P ORDER_ID`.
   - No `--confirm`: JSON preview (current order, new price, notional). Send nothing.
   - `--confirm`: submit the modification. Re-check `OMI_MAX_NOTIONAL` against the new price.
   - Live still needs `--live` AND `OMI_ALLOW_LIVE=1`.
2. Exit codes, applied to existing writes where it does not change success (exit 0) behavior:
   - 4 `data` — precondition / data error, order NOT sent.
   - 7 `rejected` — broker rejected a submitted order.
   - 8 `unconfirmed` — sent, outcome unknown (ack timeout). Callers must never auto-retry.
   - 64 `usage` or `guard` — usage, or a risk-cap / live-posture block before send.
   - Read timeouts stay 6 `timeout`. The live-port env gate stays 5 `config` (frozen CLI tests).

## API reality (verified against ibapi 3.1 in the cargo registry, not assumed)

- `ibapi::client::blocking::Client::place_order` doc (`orders/sync.rs`): **"Places or modifies an Order"**.
  There is no separate modify method. `verify_order` does not reject a reused id.
- IB TWS API "Modifying Orders": call `placeOrder` again with the same `orderId` and the same
  fields except the field being changed (price / size / tif). Changing other fields should be
  a cancel + new order. `reprice` changes the limit only.
- **Tiger is UNVERIFIED.** Implement the IB path. Do NOT fall back to cancel+replace.
  Paper `:4002` acceptance is an operator step, not a merge gate (no gateway in this environment).

## Decisions

- **D1 — same-id `placeOrder`, never cancel+replace.** 📖 ibapi 3.1 + IB modifying-orders.
  `next_order_id()` is not called on this path. A new id would be a second order.
- **D2 — preview is the default.** `--confirm` is the only transmit. Preview is read-shaped:
  it skips `require_live_write_gate` (ADR 0027). `--confirm` runs the gate BEFORE connect.
- **D3 — single-leg LMT, STK or OPT only.** MKT / BAG / other sec-types → data/4, nothing sent.
  Spreads, funds, covered calls, advanced order types stay out of scope.
- **D4 — notional is `qty × |new_price| × multiplier`, re-checked on confirm.** Paper exempt
  (existing `check_live_write_posture`). Over cap on confirm → `guard`/64, nothing sent.
  Preview of an over-cap order still returns JSON with `transmits: false` and `risk_blocked: true`.
  A present-but-invalid `OMI_MAX_NOTIONAL` fails closed (`guard`/64) before connect, both arms.
  Same mapping on `place_core` and live `option-combo` (was `config`/5). Success path unchanged.
- **D5 — write ack timeout is exit 8 `unconfirmed`, not exit 6.** Shared helper
  `submit_timeout_error` used by place, cancel, and reprice. Message names the order id,
  says UNCONFIRMED, points at `omi orders`, and says `Do NOT retry`. Read `AppError::timeout`
  stays exit 6 (frozen by `tests/read_timeouts.rs`).
- **D6 — broker rejection is exit 7 `rejected`.** `Error::Notice` with `is_order_rejection()`
  and not `is_cancellation()` (code 202 is a cancel confirmation, not a reject). An OrderStatus
  ack, including `Inactive`/`Cancelled`, stays exit 0 — success behavior unchanged; the agent
  reads `status`. A local `place_order` error before send stays data/4. A transport error
  after the call is unconfirmed/8.
- **D7 — containment.** All of this lives in `src/ib/trade.rs`. `cli.rs` only adds `RepriceArgs`.
- **D8 — docs.** AGENTS.md drops "no modify" and records the exit table. CLAUDE.md stays a
  short pointer (< 900 bytes, frozen `tests/claude_md.rs`).

## Non-scope

Advanced order types, streaming, funds, spreads, covered calls, GTC, changing qty or tif,
cancel+replace, live trading by this change.
