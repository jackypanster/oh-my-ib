# ADR 0031 — live-combo-unlock (pure-width defined-risk breaker replaces the combo lockout)

Status: accepted · 2026-07-06 · feature: live-combo-unlock · extends ADR 0030

## Context

ADR 0030 shipped the live write posture guardrail and, in D4, hard-locked ALL `option-combo` orders out
of the live path: `refuse_live_combo_on_live` (`trade.rs:247`) returns `Err` for any live combo, wired
at `trade.rs:856` BEFORE `require_live_write_gate`. The lockout was deliberate — the ADR's economic
breaker `compute_notional = qty × |limit| × 100` reads the combo's NET premium, which under-counts a
spread's real risk (NVDA 185/180 put credit @ 0.60: net-notional `$60` vs true max loss
`(185−180)×100 − 60 = $440`; a `185/175` @ 0.60 is `$940` loss but still `$60` net-notional ⇒ would pass
a `$500` cap). ADR 0030 §Alternatives-rejected parked the fix: "Notional = true risk (spread width /
margin) … Revisit only if live combo is opened." The operator is now opening a live combo (the NVDA
vertical above) ⇒ this ADR is that revisit.

## Decision

Replace the categorical combo lockout with a **combo-defined-risk breaker** that admits exactly one
bounded-risk shape — a clean 2-leg 1:1 vertical spread — onto the live path, under the UNCHANGED gate
(`--live` + `OMI_ALLOW_LIVE=1`) and the UNCHANGED cap (`OMI_MAX_NOTIONAL`, default `$500`). Every other
combo shape stays refused. The risk number is computed **from the legs' strikes only** — the
operator-typed net limit is NOT read — so a mistyped premium cannot corrupt it (premium-proof) and every
refuse path stays offline-deterministic (freezable), exactly as ADR 0030.

### D2 — risk basis = PURE WIDTH

`max_risk = |strike_a − strike_b| × 100 × qty`. This is the exchange-margin / collateral definition of a
vertical's risk and an UPPER bound on true max loss for any 1:1 vertical (credit or debit). It ignores
the premium entirely: it cannot be widened by a mistyped credit, and it unifies credit & debit under one
formula. Consequence (accepted by operator): the NVDA 185/180 (width `$5` ⇒ risk exactly `$500`) sits AT
the `$500` default cap — `check_live_write_posture` uses `>` (not `>=`), so `500 > 500` is false ⇒ passes
with zero headroom; wider or cheap-debit verticals are conservatively over-counted and need a per-command
`OMI_MAX_NOTIONAL` bump. Rejected: exact max-loss (`credit: (width−credit)×100×qty` / `debit:
debit×100×qty`) — truer, but re-trusts the typed premium (a mistyped credit > width yields a negative
"risk" that passes unconditionally) and needs clamp guards; premium-proof simplicity wins for a
fat-finger breaker.

### D3 — admitted shape = CLEAN 2-LEG 1:1 VERTICAL only

A live combo is admitted iff ALL seven hold (else `Err(reason)` ⇒ refused). Predicate grounded on
`LegSpec` (`trade.rs:684`: `action`/`ratio`/`symbol`/`expiry`/`strike`/`right`, all normalized):

1. exactly 2 legs (`specs.len() == 2`) — else refused (calendars/butterflies/condors are paper-only);
2. same underlying (`specs[0].symbol == specs[1].symbol`) — already enforced upstream (`trade.rs:831`),
   re-asserted here so the seam is self-contained and independently freezable;
3. same expiry (`specs[0].expiry == specs[1].expiry`) — else a calendar/diagonal ⇒ refused;
4. same right (both `"C"` or both `"P"`) — else a diagonal/straddle-ish shape ⇒ refused;
5. opposite actions (one `BUY`, one `SELL`; parse guarantees each ∈ {BUY,SELL}, so `action[0] !=
   action[1]`) — else not a spread ⇒ refused;
6. both `ratio == 1` — else a ratio/backspread (unbounded/undefined risk) ⇒ refused;
7. distinct strikes (`specs[0].strike != specs[1].strike`) — else zero width ⇒ refused.

This is a bounded geometry check, NOT the general risk engine ADR 0030 rejected. `qty` is trusted whole
≥ 1 (validated upstream, `trade.rs:837`); multiplier is hard-coded `100.0` (combos are option-only).

### Seams (pure = FROZEN; wired = review-by-reading)

Pure, FROZEN (new — tested in `tests/live_combo_unlock.rs`):

```rust
// D3 clean-vertical predicate + D2 width×100×qty. Premium-proof: the net limit is NOT a parameter.
// Ok(max_risk) for a clean 2-leg 1:1 vertical; Err(reason) for every other shape.
pub fn combo_live_max_risk(specs: &[LegSpec], qty: f64) -> Result<f64, String>
```

Reused, already FROZEN (in `tests/live_write_guardrail.rs`, ADR 0030 — NOT re-frozen, only called):

```rust
resolve_max_notional(raw: Option<&str>) -> Result<f64, String>   // cap: None⇒$500, bad⇒Err (fail-closed)
check_live_write_posture(is_live, is_mkt, notional, cap) -> Result<(), String>  // >cap⇒Err, ==cap⇒Ok
```

Wired (gateway fn `option_combo`, `trade.rs:852-857` — review-by-reading, NOT frozen). Replace the single
`refuse_live_combo_on_live(...)` line with, still INSIDE `if !cfg.preview` and BEFORE
`require_live_write_gate`:

```rust
let is_live = cfg.port == LIVE_PORT;
if is_live {
    let max_risk = combo_live_max_risk(&specs, args.qty).map_err(|m| AppError::config(m, ctx))?;
    let cap = resolve_max_notional(std::env::var("OMI_MAX_NOTIONAL").ok().as_deref())
        .map_err(|m| AppError::config(m, ctx))?;
    check_live_write_posture(true, false, Some(max_risk), cap).map_err(|m| AppError::config(m, ctx))?;
}
require_live_write_gate(cfg)?;
```

`&specs` is valid here — `specs` is owned until its `into_iter` consume at `trade.rs:872`, well after this
block. `is_mkt` is structurally `false` (combos are always LMT, `build_combo_order`). Paper
(`is_live == false`) skips the whole risk/cap block ⇒ any combo shape still places on `:4002`. Preview is
outside `if !cfg.preview` ⇒ untouched.

### Ordering — posture BEFORE the gate (deliberate; differs from `place_core`)

`place_core` (ADR 0030) runs the gate FIRST, then the posture. `option_combo` runs the combo posture
BEFORE the gate — preserving the lockout's original pre-gate placement. Rationale: a shape/cap refusal
holds REGARDLESS of `OMI_ALLOW_LIVE`, so it must report first; running the gate first would tell the
operator "set `OMI_ALLOW_LIVE`" for a combo that is structurally refused even with the env set —
misdirection. So: non-vertical / over-cap ⇒ the posture message (before the gate); a clean within-cap
vertical missing the env ⇒ falls through to the gate's `OMI_ALLOW_LIVE` message (correct — it is an
admitted shape, only the env is missing).

### D5 — over-cap message reuses `check_live_write_posture` verbatim

The over-cap refuse says `"live notional {n} exceeds cap {cap} — raise OMI_MAX_NOTIONAL to override"`.
For a combo `n` is the max-risk-notional; the word "notional" is retained (it IS a notional/risk figure,
and CC's action — smaller spread or raise cap — is unchanged by the wording). REUSING the frozen fn keeps
the new frozen surface to `combo_live_max_risk` ONLY; a combo-worded "max risk" message was rejected — it
would duplicate the `>cap`/`==cap` compare and add frozen surface for a cosmetic gain.

### D6 — `refuse_live_combo_on_live` stays defined + re-exported + frozen; only UNWIRED

`tests/live_write_guardrail.rs` (spec-rev `817c7d8`, feature DONE/merged) freezes
`refuse_live_combo_on_live`. Deleting it breaks another card's frozen spec — forbidden (AGENTS.md hard
invariant). So the fn stays at `trade.rs:247`, stays in the `mod.rs:45` re-export, its frozen test stays
byte-identical and green. This ADR only REMOVES its sole production call (`trade.rs:856`). It remains
`pub` + referenced by that integration test ⇒ no `dead_code` warning. `tests/live_write_guardrail.rs` is
NOT edited.

## Consequences

- A live combo is now bounded: only a clean 2-leg 1:1 vertical, risk = full width × 100 × qty ≤ cap.
  Every other shape (≥3 legs, calendar, diagonal, straddle, ratio, same-strike, cross-underlying) stays
  refused with exit 5 before connect.
- Premium-proof: the risk breaker cannot be widened by a mistyped net limit.
- The `$500` default admits a 1-lot ≤ `$5`-wide vertical (the NVDA order, at the boundary); a `$10`-wide
  or multi-lot needs an explicit `OMI_MAX_NOTIONAL` (per-command, auditable — mirrors `OMI_ALLOW_LIVE`).
- Contained: the ADR 0030 gate, cap seams, `place_core`, single-leg/STK, `option-close`, `cancel`,
  paper, and preview are all unchanged; new code is one pure seam + a re-export + the `option_combo`
  rewire; `refuse_live_combo_on_live`'s frozen spec is untouched.

## Freeze coverage

- **FROZEN** (`tests/live_combo_unlock.rs`, pure `combo_live_max_risk`): Ok — put credit vertical
  (SELL 185P / BUY 180P, qty 1) ⇒ `500.0`; call vertical 250/240 qty 2 ⇒ `2000.0`; a debit vertical
  (BUY 180P / SELL 185P, qty 1) ⇒ `500.0` (SAME as the credit — proves D2 premium-proof/credit-debit
  unified). Err — 1 leg (`len<2`); 3 legs; diff expiry; diff right; same action; ratio 2; equal strikes;
  diff underlying. (`combo_live_max_risk` is the ONLY new frozen surface.)
- **REVIEW-BY-READING** (not frozen — gateway wiring): the `option_combo` rewire — `is_live` derivation;
  `combo_live_max_risk(&specs, args.qty)` before connect; `resolve_max_notional` env read;
  `check_live_write_posture(true, false, Some(risk), cap)`; posture-before-gate ordering; every
  `Err → AppError::config` (exit 5); paper/preview/`refuse_live_combo_on_live`-unwired unchanged;
  `tests/live_write_guardrail.rs` byte-identical.
- **OPERATOR LIVE ACCEPTANCE** (post-merge, the trial): the NVDA 185/180 live command with
  `OMI_ALLOW_LIVE=1` on `:4001` actually places (would place a real order — never unit-tested); a
  non-vertical / over-cap live command refuses exit 5 with NO order (verifiable with `:4001` down too).

## Alternatives rejected

- **Exact max-loss risk.** Re-trusts the typed premium (fat-finger hole; needs clamps) — see D2.
- **Delete the lockout, reuse net-premium `compute_notional`.** The safety regression this ADR exists to
  prevent (combo-risk-blind) — see Context.
- **A separate combo cap env (`OMI_MAX_COMBO_RISK`).** Rejected — one cap key (`OMI_MAX_NOTIONAL`) is
  simpler and already the operator's mental model; combos and single-legs share the economic ceiling.
- **Combo-specific over-cap message.** Rejected — cosmetic; would add frozen surface (see D5).
- **Unlock condors/butterflies too (defined-risk multi-leg).** Deferred — bounded but needs a
  per-structure max-loss model; ship the vertical first (the operator's actual order), revisit on demand.
