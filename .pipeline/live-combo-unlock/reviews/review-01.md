# review-01 — live-combo-unlock (card 01)

Stage: review · feature: live-combo-unlock · PR #28 · merged squash `6de5354` to main · 2026-07-06

## Verdict: ACCEPT → MERGED

Writer = OMP (π, GLM-5.2). Reviewer = codex (gpt-5.5 xhigh). Writer ≠ reviewer. cc merged after
operator confirm (gated on CodeRabbit SUCCESS).

## Gates

- **codex semantic review (ACCEPT, no findings).** Read ADR 0031 + ADR 0030 + card 01 + both frozen
  specs + CONTRACT.md/PRD/CONTEXT/journal, then the diff. Confirmed: the live combo path feeds the
  width-based `combo_live_max_risk(&specs, args.qty)` into the cap check (NOT the blind net-premium
  notional — the #1 regression check); `refuse_live_combo_on_live` kept defined + re-exported, only
  unwired; `tests/live_combo_unlock.rs` + `tests/live_write_guardrail.rs` byte-identical
  (`git diff --exit-code` clean); diff is exactly the two impl-path files. Self-verified
  `cargo build` + `cargo test` + `cargo clippy --all-targets -- -D warnings` all pass.
- **cc freeze gate.** `git diff e2f2b17..origin/feat/live-combo-unlock -- tests/live_combo_unlock.rs
  tests/live_write_guardrail.rs` EMPTY — frozen specs untouched since spec-rev `e2f2b17`.
- **cc containment gate.** Diff = `src/ib/trade.rs` + `src/ib/mod.rs` only (+43/-5). No `tests/` change;
  `refuse_live_combo_on_live` still defined at `trade.rs:247` and NO longer called in `src/` (unwired).
- **CodeRabbit: pass** ("Review completed"). mergeStateStatus CLEAN.
- **Trunk full-verify post-merge (6de5354):** `cargo build` OK · full `cargo test` all green
  (`live_combo_unlock` 11/11; `live_write_guardrail` still green) · `cargo clippy --all-targets -D
  warnings` clean.

## What shipped

`combo_live_max_risk` (pure-width `|Δstrike|×100×qty`, premium-proof) replaces the ADR 0030 D4 combo
lockout's call site in `option_combo`; only a clean 2-leg 1:1 vertical (same underlying/expiry/right,
opposite actions, ratio 1, distinct strikes) is admitted onto live, gated by `--live` +
`OMI_ALLOW_LIVE=1` and capped by `OMI_MAX_NOTIONAL` (default $500) via the reused
`check_live_write_posture`. Every other combo shape refuses (exit 5) before connect. Paper/preview
unchanged.

## OPERATOR LIVE ACCEPTANCE (post-merge — not asserted; the trial itself)

Flip Tiger gateway to live `:4001`. Then, on `:4001`:

- **REFUSE, no order** (verifiable with `:4001` down too — offline): `omi --live option-combo` for a
  185/175 vertical (width $10 ⇒ risk $1000 > $500) ⇒ exit 5 "notional exceeds cap"; a non-vertical
  (3-leg / calendar / straddle / ratio) ⇒ exit 5 "2-leg vertical spreads only".
- **PASS the guardrail → place** (the first live combo, real money): `OMI_ALLOW_LIVE=1 omi --live
  option-combo` for the NVDA 20260715 185/180 put credit (width $5 ⇒ risk $500 == default cap ⇒ passes;
  `>` is the refuse). Net credit ~$60, max risk ~$440 ex-fees. Set `OMI_MAX_NOTIONAL=600` for headroom
  if desired. Confirm `omi --live orders` shows the BAG; cancel to verify-by-cancel if not intending to
  hold.
- **Paper unaffected:** `omi option-combo` on `:4002` still places any combo shape.

## LIVE-ACCEPTANCE RESULT (2026-07-06)

First live combo attempted end-to-end on `:4001` (account `U20230856`), operator-authorized. The exact
order was preview-confirmed first (`--live --preview`, `transmits:false`): BAG `SELL 185P` (conid
897191251) / `BUY 180P` (conid 897191240), `limit -0.6` credit, qty 1, notional (net-premium) $60.

Real placement (`OMI_ALLOW_LIVE=1 omi --live option-combo --action BUY --qty 1 --limit -0.60 --leg
"SELL 1 NVDA 20260715 185 P" --leg "BUY 1 NVDA 20260715 180 P"`) → **REJECTED by the gateway:
`{"error":{"code":"data","context":"option-combo","message":"order stream: [460] No trading
permissions."}}`**. Read of `code=data` (order-stream, i.e. broker-side): the order PASSED the ADR 0031
width-risk breaker ($500 == cap ⇒ allowed) + the live gate + connected to `:4001` and reached the broker
— the guardrail chain is validated end-to-end. The BROKER refused: account `U20230856` lacks
options/spread trading entitlement on Tiger. **NO order placed** (post-check `omi --live orders` shows
only the pre-existing `BUY 1 NVDA LMT 195`, order_id 2). BLOCKER is a Tiger account permission, NOT a
code fix — re-run the identical command once options/combo trading is enabled on the account.
