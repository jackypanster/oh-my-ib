# CONTEXT — preview-readonly

Domain glossary + the (now resolved) reference-behavior record for the read-only preview.

## Glossary

- **read-only preview** — `omi <verb> … --preview` builds the same `(Contract, Order)` a real order
  would, RESOLVES the contract via `client.contract_details` (STK/single-leg option) or reuses the
  already-resolved conids (combo/close), and returns a confirm envelope. It calls **NO `place_order`** —
  it cannot become an order. Replaces the order-preview whatIf path.
- **`transmits: false`** — explicit envelope marker telling the consumer (cc now; Hermes later) that this
  call placed nothing. The safety contract, machine-readable.
- **notional** — `qty × limit × multiplier` (STK 1, OPT 100, combo `|net_limit|`×100). The "max principal"
  a later real order would commit; the read-only substitute for margin (which needs whatIf → transmits).
- **read-shaped gate** — preview reuses the READ gate (just `--live` for the port), NOT
  `require_live_write_gate`. Justified because preview is now a pure read (no submission). The preview
  branch sits BEFORE `require_live_write_gate` in each verb.
- **containment** — `place_order`/`cancel_order` appear ONLY on the real order path in `src/ib/trade.rs`.
  The read-only preview must not add any. Review greps this.

## Reference behavior — Tiger whatIf (RESOLVED: R1 refuted)

| # | claim | tier | outcome |
|---|---|---|---|
| R1 | `Order.what_if=true` ⇒ IB computes margin WITHOUT transmitting | ✅ **probed → REFUTED** (2026-07-05, acct U20230856) | Tiger TRANSMITS the whatIf order (a real resting order appeared; price-band-rejected; zero financial impact). **The design consequence: drop `place_order(what_if)` entirely — this feature does exactly that.** |

There is **no remaining unverified external premise**: the read-only preview depends only on
`client.contract_details` — a standard read the codebase already uses reliably (`omi contract`,
`option_quote`). The safety property ("preview does not transmit") is now **structural** (no
`place_order` in the path), not a bet on gateway behavior — verified by review (containment grep) +
cc live-acceptance (`omi --live orders` empty after a preview).

**Live-acceptance (operator directive: cc+omi only, Hermes/TG deferred):** after impl,
`omi --live buy AAPL 1 --limit 1 --preview` must return the resolved-contract envelope AND
`omi --live orders` must be EMPTY — the acceptance that REVERSES the R1-refuted finding for the fixed path.
