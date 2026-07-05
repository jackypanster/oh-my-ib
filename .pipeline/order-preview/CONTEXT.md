# CONTEXT — order-preview

Domain glossary + the reference-behavior risk register for the whatIf preview feature.
Grounds terms so a cold task/impl node uses them consistently.

## Glossary

- **preview** — a non-transmitting order simulation. `omi <verb> … --preview` builds the exact same
  `(Contract, Order)` a real order would, flips `what_if=true`, sends it as a whatIf query, and
  returns the uniform preview envelope. It NEVER becomes a resting order (assuming the gateway honors
  `what_if` — see §Reference behavior).
- **whatIf / `what_if`** — the IB TWS API `Order.what_if: bool` flag (ibapi `orders/mod.rs:322`,
  default `false` at `:562`). When `true`, TWS/IB computes margin + commission impact and returns them
  on `OrderState` WITHOUT placing the order. The write path's default `what_if=false` is a load-bearing
  row already in `docs/write-path-semantics.md`; preview flips it to `true` for the preview path ONLY.
- **preview envelope** — the single uniform JSON shape returned for all six verbs (arch.md §Uniform
  envelope). Keys are stable; margin/commission values are `null` when the gateway does not populate
  them. This is hermes's confirm card.
- **the gate** — `require_live_write_gate(cfg)` (trade.rs:143): on the live port (4001), rejects
  unless `OMI_ALLOW_LIVE=1`. Preview reuses it **unchanged** (branch is after the gate) → preview is
  never more permissive than the real order it previews.
- **live-acceptance** — operator-run validation on `:4001` (no pipeline stage reaches a gateway). The
  observable that confirms/refutes the `what_if` premise: a far-from-market `--preview` returns the
  envelope AND `omi --live orders` shows NO resting order.

## Reference behavior — Tiger/IB `what_if` semantics (risk register)

The frozen tests freeze our UNDERSTANDING of `what_if`; they cannot reach a gateway, so the two rows
below are guarded ONLY by live-acceptance, not by any red test. Shipped `⚠️` = the risk register (same
pattern as `docs/write-path-semantics.md`).

| # | claim the code relies on | reference / source | our value | verification tier | probe (operator, `:4001`) |
|---|---|---|---|---|---|
| R1 | `Order.what_if=true` ⇒ IB returns margin/commission on `OrderState` and does **NOT** transmit | IB TWS API `reqWhatIfOrder` semantics; ibapi `OrderState` margin/commission fields (`mod.rs:1275+`) | `what_if=true` on the preview branch only | ⚠️ unverified on **Tiger** (Tiger is TWS-API-compatible but whatIf depth is unconfirmed) | `omi --live buy AAPL 1 --limit <FAR-below-market> --preview` → envelope returns; then `omi --live orders` shows NO order for AAPL. Present ⇒ **R1 refuted** (Tiger ignored what_if) ⇒ do NOT relax the gate; open a fix feature. Absent ⇒ **R1 ✅**. |
| R2 | Even honoring `what_if`, Tiger may leave margin/commission empty | ibapi `OrderState` fields are all `Option<f64>` | envelope emits `null` for empty fields | ⚠️ unverified (non-blocking by design) | same probe: inspect envelope `margin`/`commission`. All `null` ⇒ preview still valid as an echo+resolved-contract confirm card; margin numbers are a bonus. No fix needed. |

Fallback posture (both rows): the feature is designed to **ship value regardless** — the
resolved-contract + order echo are always present; margin/commission are best-effort. The catastrophe
case is R1-refuted (a "preview" that transmits); it is contained in v1 by (a) the identical write gate
and (b) the far-from-market limit in the acceptance protocol (a transmitted order rests, never fills,
stays visible/cancellable).

**Durable-doc note (operator-owned, non-blocking, mirrors write-path-semantics D2):** after
live-acceptance resolves R1/R2, record the dated result as a row in `docs/write-path-semantics.md`'s
risk register. This is NOT an impl-path and NOT frozen — it is a one-line doc edit the operator makes
post-acceptance. `pipeline-task` does not gate on it.
