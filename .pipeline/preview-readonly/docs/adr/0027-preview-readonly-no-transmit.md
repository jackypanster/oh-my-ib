# ADR 0027 — preview is read-only (no place_order); read-shaped gate

Status: accepted (2026-07-05, feature preview-readonly)
Supersedes the whatIf mechanism of ADR 0026 (order-preview). Relates: ADR 0017 (write-path-safety).

## Context

order-preview (ADR 0026) implemented `--preview` via `place_order(order.what_if=true)`. Live-acceptance
on the Tiger gateway (acct U20230856, 2026-07-05) REFUTED premise R1: Tiger TRANSMITS whatIf orders (a
real resting order appeared; contained safely, zero financial impact). So the whatIf mechanism is unsafe.

## Decision

1. **The preview path calls NO `place_order`.** Resolve the contract via `client.contract_details`
   (STK/single-leg option) or reuse already-resolved conids (combo/close), echo the built order, compute
   notional. `preview_with_client` (place_order + what_if) is removed. Safety ("preview does not transmit")
   becomes **structural**, not a bet on gateway whatIf semantics.
2. **Read-shaped gate.** The preview branch runs BEFORE `require_live_write_gate`; `--preview` needs
   `--live` only (no `OMI_ALLOW_LIVE`), consistent with the read commands it now resembles. Flips ADR 0026's
   fail-safe gate — justified because there is no longer any `place_order` to guard.
3. **Envelope** drops `what_if`/`margin`/`commission`/`status`; adds `transmits:false` + `notional`
   (qty×limit×multiplier; STK 1, OPT 100) + a `note` that margin/commission are unavailable on this gateway.
4. **Module stays in `src/ib/trade.rs`** — minimal churn; reuses the pure builders; the containment
   invariant (`place_order`/`cancel_order` only in trade.rs) is unaffected by a read-only addition.

## Consequences

- Breaking change to the `--preview` envelope (old whatIf shape gone). Acceptable: the old shape was not
  safely usable (it transmitted).
- `margin`/`commission` are no longer available (they require whatIf, which transmits on Tiger). notional
  is the read-only substitute.
- The real order path is untouched; `place_order`/`cancel_order` remain only on it.
- The frozen `tests/order_preview_command.rs` asserts the OLD behavior (what_if:true; gate=config) and is
  REPLACED by task, not preserved.

## Rejected alternatives

- **Keep whatIf but add a post-place `orders` check + auto-cancel** — still transmits (a fill could beat
  the cancel); fragile; rejected. No `place_order` is the only safe design.
- **Extract to `src/ib/preview.rs`** — cleaner separation, but more churn for a fix; deferred. Containment
  does not require it (preview has no write calls).
- **Ungate nothing / keep fail-safe gate** — unnecessary friction now that preview is a pure read
  (human-confirmed read-shaped).
