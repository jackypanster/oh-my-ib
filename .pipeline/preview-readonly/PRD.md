# PRD — preview-readonly (fix: `--preview` must never transmit)

Status: prd (2026-07-05, feature preview-readonly)
Origin: live-acceptance of order-preview (PR #23) on the Tiger gateway REFUTED premise R1.
Branch (eventual PR): `feat/preview-readonly`. Trunk: `main`.

## Problem

- order-preview (PR #23, merged `c5d9abb`) implemented `--preview` via `place_order(order.what_if=true)`
  (`src/ib/trade.rs` `preview_with_client:423`, `order.what_if = true` at `:432`).
- **R1 REFUTED — empirical, live account U20230856, 2026-07-05 (Read-Only API OFF):**
  `OMI_ALLOW_LIVE=1 omi --live buy AAPL 1 --limit 1 --preview` returned a preview envelope BUT
  `omi --live orders` then showed a **REAL resting order** (order_id 1, AAPL Buy 1 @ 1.0 LMT DAY).
  **Tiger does NOT honor `what_if` as a no-transmit query — it TRANSMITS a real order.** Tiger's own
  price-band control then rejected the $1 order; full accounting after = clean (no open/completed order,
  no execution, no position) → **zero financial impact** (the far-limit + 1-share + immediate
  `orders`-check safety chain held exactly as designed). ✅ human-confirmed (cc ran the probe with the operator).
- Consequence: `--preview` as shipped is NOT a safe preview — it places a real order. A realistic limit
  (e.g. 290, ~1.5% below the ~294 market) would rest a fillable real order.

## Goal

Re-implement `--preview` so it **NEVER submits an order**. Build the confirm card from **read-only**
data: resolve the contract via `client.contract_details` (📖 `src/ib/contract.rs` — the same read
`omi contract` uses; returns real `conid` + validated identity), echo the order params from the built
`Order`, compute notional. **Remove** the `place_order(what_if)` path (`preview_with_client` + the
`what_if=true` flag). Core value preserved: the resolved contract catches LLM misparse of
symbol/expiry/strike/right (the #1 fat-finger risk); echo + notional show exactly what a later real
order would send — with **zero transmit risk**. This is the "ship value even if Tiger ignores whatIf"
fallback the original PRD anticipated (order-preview CONTEXT.md R1); the impl took the wrong (place_order) path.

## Decisions (provenance-tagged)

1. **Read-only via `contract_details`, NO `place_order`/`what_if`.** ✅ human-confirmed (fix direction)
   + 📖 code-verified (`contract.rs` `client.contract_details`; `trade.rs:423/432` is the path to remove).
2. **Gate = read-shaped: `--live` only, NO `OMI_ALLOW_LIVE`.** ✅ human-confirmed (2026-07-05). Preview
   is now a pure read (like `omi contract`/`quote`, which are ungated for the write-gate), so it joins
   them. FLIPS the order-preview decision (that was fail-safe because `place_order` could transmit;
   there is no `place_order` now).
3. **Full envelope** (✅ human-confirmed): `preview:true`, **`transmits:false`** (explicit non-submit
   marker), `action`, `contract` {symbol, conid, exchange, currency, sec_type, long_name; + expiry/
   strike/right for options}, `order` {type, qty, limit}, **`notional`** = qty × limit × multiplier
   (STK 1, OPT 100) + `notional_currency`, and margin/commission **dropped** (a `note`:
   "unavailable on this gateway — whatIf transmits"). Drops the old `what_if`/`margin`/`commission`/
   `status` keys (breaking change — acceptable, the old shape is not safely usable).
4. **All six order verbs** (✅ human-confirmed, carried): buy/sell/option-buy/option-sell/option-combo/option-close.
5. **Real order path byte-unchanged; `place_order`/`cancel_order` stay ONLY on the real
   buy/sell/cancel/option paths** (containment). ✅ + 📖 (`trade.rs`).
6. **Module placement (trade.rs vs a new read-only module)** — ⚠️ deferred to ARCH. Preview is no
   longer a write, so the AGENTS.md "write code ONLY in `src/ib/trade.rs`" rule no longer forces it
   there; but it reuses the pure builders (`build_stk_order` etc., in trade.rs). Arch decides: keep in
   trade.rs (minimal churn) or extract to `src/ib/preview.rs` importing the builders. No user impact.

## Success criteria (decision-complete)

1. `omi <verb> … --preview` performs **NO order submission** — the preview path calls only
   `contract_details` (read); `preview_with_client`/`place_order(what_if)` is REMOVED.
2. Returns the full read-only envelope (Decision 3), stable keys, `transmits:false`, notional correct
   (STK ×1, OPT ×100).
3. Gate: preview reachable with **`--live` alone** (no `OMI_ALLOW_LIVE`); on a dead port ⇒ a
   `connection` error (past the gate), NOT a `config` error.
4. Real order path byte-unchanged; containment grep: `place_order`/`cancel_order` appear ONLY on the
   real order path in `src/ib/trade.rs`, never in the preview path.
5. All six verbs covered.
6. `cargo build` · `cargo clippy --all-targets -- -D warnings` · `cargo test` GREEN.
7. **Live-acceptance (cc runs omi directly — operator: for now ONLY cc+omi, Hermes/TG deferred):**
   `omi --live buy AAPL 1 --limit 1 --preview` returns the resolved-contract envelope AND
   `omi --live orders` is **EMPTY** — RE-validates R1 as FIXED (genuinely no transmit). This acceptance
   reverses the R1-refuted finding.

## Freeze note (task will re-write the existing spec)

`tests/order_preview_command.rs` is the current spec and asserts the OLD behavior — `what_if:true` in the
envelope (lines 60/64) and gate = `config` error (`preview_on_live_without_env_is_config_error…`, :157).
The fix CHANGES both: envelope → `transmits:false` + notional (no `what_if`/margin/commission); gate →
`connection` (read-shaped). Task re-freezes this file (or a replacement) for the NEW behavior. Frozen:
the read-only envelope shaper (pure, over constructed resolved-contract + order + notional), notional
math, and the `--preview` read-shaped gate (black-box: `--live --preview` without env on a dead port ⇒
`connection`, not `config`). Review-by-reading (needs a live gateway): the `contract_details` wiring +
that the preview path contains no `place_order` (containment grep).

## Scope

Rewrite the `--preview` path to read-only; the new envelope + notional; the read-shaped gate; all six verbs.

## Non-scope (explicit)

- **NO margin/commission** (impossible read-only on Tiger — whatIf transmits).
- Real order path unchanged (buy/sell/cancel/option orders + their gate).
- **Hermes/TG wiring DEFERRED** — operator directive 2026-07-05: mature the CLI first, use ONLY cc+omi
  for now; discuss Hermes+TG when stable.
- The old `what_if`-based preview envelope is dropped (breaking change, acceptable).

## Handoff → pipeline-arch

Decide Decision 6 (module placement) against the codebase; design the read-only preview fn (replace
`preview_with_client`) using `client.contract_details`; the exact envelope key set + notional/multiplier
source (STK vs OPT contract multiplier); how the three branch sites (place_core:481, option_combo:820,
option_close:1072) call the new read-only path; and confirm containment (no `place_order` in preview).
Emit arch.md + CONTEXT.md + ADR. Do NOT touch src/tests.
