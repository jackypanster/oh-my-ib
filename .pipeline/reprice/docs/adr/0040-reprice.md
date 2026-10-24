# ADR 0040 — reprice: same-id placeOrder, ftnn exit codes

Status: accepted (operator-approved 2026-10-22). Amends ADR 0017 §5 (v1 surface "no modify")
and ADR 0017 §3's placement-timeout exit for writes only. Does not amend ADR 0012 (read
timeout stays exit 6).

## Context

`omi` writes are place + cancel. Modifying a resting limit means cancel + a new order, which
drops queue position and can double-fire when the cancel is unconfirmed. The sister CLI
`ftnn` modifies in place and uses exit 4 / 7 / 8 / 64 so an agent can tell "not sent" from
"broker said no" from "we do not know, do not retry".

## Decision

1. **Modify = `place_order` with the same order id.** ibapi 3.1 documents `place_order` as
   "Places or modifies". The IB TWS API modifying-orders page is the same rule. `reprice`
   copies the open order and changes the limit only (`transmit` forced true). It does not
   call `next_order_id()`. It does not cancel.
2. **Tiger is unverified.** Ship the IB path. Paper `:4002` acceptance is required before an
   operator relies on it. If Tiger rejects or opens a second order, record that; do not add
   a cancel+replace fallback in this feature.
3. **Preview by default.** `--confirm` is the only transmit. Preview skips the live env gate.
   Confirm runs it first. Both arms fail closed on an invalid `OMI_MAX_NOTIONAL`.
4. **Notional cap uses the new price** (`check_live_write_posture`, paper exempt). Over cap
   on confirm is `ErrorKind::Guard`, exit 64, code `guard`, nothing sent. The same guard
   mapping replaces `AppError::config` for the notional/MKT/combo-risk refuses in
   `place_core` and live `option-combo`. The `OMI_ALLOW_LIVE` gate stays `config`/5 and, on
   `option-combo`, runs BEFORE the width-cap check: a default-cap vertical would otherwise
   report `guard` and hide the gate, which frozen `option-combo --live` asserts as `config`.
5. **After send:** broker rejection notice → exit 7 `rejected`. Ack timeout or transport
   loss → exit 8 `unconfirmed`, message forbids automatic retry. Before send: data/4.
   An order-status ack remains exit 0 (success behavior unchanged).
6. **Scope:** open single-leg STK/OPT LMT. Other shapes are data/4.
7. **Containment:** write calls stay in `src/ib/trade.rs` (ADR 0017).

## Consequences

- AGENTS.md no longer says modify is out of scope. The exit table lives there.
- Agents must not auto-retry exit 8. Exit 7 means the broker answered no. Exit 4 and 64
  mean nothing was sent, so a corrected retry is safe.
- A future qty/tif modify reuses `reprice_order`'s same-id rule; it is not this feature.
