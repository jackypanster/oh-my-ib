# ADR 0019 — Options read path: two drain classes, model-only best-effort greeks, first-row conid

Status: accepted (options-read; Phase 2 step 2 — READ-ONLY, no write-path impact)

## Context

`option-chain` (reqSecDefOptParams) and `option-quote` (option snapshot + greeks) are the
first options-data commands. Three questions had real alternatives: how to bound each
stream, which OptionComputation rows count as "the greeks", and how to resolve the
underlying conid. All three freeze into the output contract, so they are pinned here.

## Decision

1. **Chain drain = End-bounded + timeout-wrapped** (D1). reqSecDefOptParams terminates via
   `SecurityDefinitionOptionParameterEnd`, which the ibapi subscription surfaces as a clean
   iterator end (source: stream_decoders.rs:50 → subscriptions/sync.rs:171-188). But the
   reqCompletedOrders precedent (ADR 0016, live-proven) says some gateway builds/states
   never send their End — so the drain uses `timeout_iter_data(TAKE_FIRST_TIMEOUT)` with
   Instant-classified `None` arms: starved window ⇒ exit-6 `timeout` envelope; instant
   `None` ⇒ End received ⇒ success. The command can never hang.
2. **Quote drain = SnapshotEnd-bounded, bare** (D2). option-quote reuses the quote.rs
   snapshot drain class unwrapped — `SnapshotEnd` is request-id-routed and has no observed
   wedge; wrapping it would diverge the two snapshot commands for no evidence. (Mirrors the
   deliberate quote.rs:44-45 non-wrap.)
3. **Greeks = model rows only, last-write-wins, best-effort** (D3). Of the OptionComputation
   rows (bid 10 / ask 11 / last 12 / model 13 / custom 53 / delayed 80–83), ONLY
   `ModelOption`(13) and `DelayedModelOption`(83) populate `greeks` — the numbers TWS shows
   as "the greeks". Bid/ask/last computations are per-side IVs, not the model surface;
   emitting them would triple the schema for marginal agent value. If several model rows
   arrive before SnapshotEnd the LAST wins. If none arrives the `greeks` key is ABSENT and
   the output is still a success — under delayed+snapshot some data farms never push
   computations, and failing there would make the command useless on exactly the default
   (delayed) path. Inside `greeks`, only Some-valued fields appear
   (`implied_volatility, delta, gamma, vega, theta, option_price, underlying_price`;
   `present_value_dividend`/`tick_attribute` excluded as noise).
4. **Underlying conid = FIRST contract_details row** (D4). `Contract::stock(sym)` (SMART/USD
   builder defaults — quote/contract parity) → `contract_details` → first row's conid;
   empty ⇒ `not_found`. Deterministic and matches the gateway's primary-listing-first
   ordering; a symbol-ambiguity flag can come later if live use ever surfaces one.

## Rationale

- One timeout vocabulary: both new bounds reuse `TAKE_FIRST_TIMEOUT` + the ADR 0016 wait
  pattern — no new constants, no new error codes.
- Model-only greeks keeps the schema stable and the freeze small; best-effort (absent key,
  never an error) deforms the design around the feature's most fragile assumption instead
  of assuming it away (PRD D3).
- First-row conid is the smallest deterministic rule; erroring on multi-row would break the
  common case (US stocks resolve to one primary row).

## Consequences

- Agents detect greeks availability by key presence — documented, deterministic.
- A wedged reqSecDefOptParams surfaces as the familiar exit-6 timeout envelope.
- If live use ever needs bid/ask-side IVs or multi-listing disambiguation, both are
  additive (new keys / new flag), not breaking.
