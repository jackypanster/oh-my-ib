# PRD — live-combo-unlock

Stage: prd · feature: live-combo-unlock · repo: jackypanster/oh-my-ib · branch: main
Author: cc. Follows ADR 0030 (live-write-guardrail, merged PR #27 / a843a08). Live combo BLOCKER:
operator wants to place a real-money defined-risk vertical (NVDA 20260715 185/180 put credit spread,
net credit 0.60) that the ADR 0030 combo lockout currently refuses.

## Problem

ADR 0030 D4 hard-locks ALL `option-combo` orders out of the live path: `refuse_live_combo_on_live`
(`trade.rs:247`) fires BEFORE the gate on `!cfg.preview` (wired at `trade.rs:856`) ⇒ `omi --live
option-combo` always exits 5 "combo is paper-only". The lockout was deliberate: the ADR's ONLY economic
breaker is `compute_notional = qty × |limit| × 100`, and for a combo the sign-free net limit is the NET
premium, which **under-counts a spread's real risk**. For NVDA 185/180 put credit @ 0.60: net-notional =
`1 × |0.60| × 100 = $60`, but true max loss = `(185−180)×100 − 60 = $440`. A `185/175` @ 0.60 credit is
$940 max loss but still $60 net-notional — it would sail through a $500 cap. So the ADR locked combos out
entirely rather than ship a breaker blind to combo risk ("Revisit only if live combo is opened" — ADR
0030 §Alternatives rejected). The operator is now opening live combos ⇒ that revisit is due.

## Goal

Replace the combo hard-lockout with a **combo-defined-risk breaker** so a bounded-risk vertical spread
can reach the live path under the SAME gate (`--live` + `OMI_ALLOW_LIVE=1`) and the SAME cap
(`OMI_MAX_NOTIONAL`, default $500), while every unbounded / undefined-risk combo shape stays refused.
The risk number is computed **offline, from the legs' strikes only** (premium-proof) ⇒ every refuse path
stays offline-deterministic ⇒ freezable, exactly like ADR 0030.

## Decisions (provenance-tagged)

- **D1 — REPLACE the lockout, do not delete-and-fall-through.** ✅ human-confirmed (/think 2026-07-06).
  The combo live path MUST be guarded by a defined-risk cap. Simply deleting the lockout and letting
  combos flow into the net-premium `check_live_write_posture` is a safety REGRESSION (blind to combo
  risk — the $940 case above passes a $500 cap). This is codex review's #1 semantic check.

- **D2 — risk basis = PURE WIDTH (premium-proof).** ✅ human-confirmed (grill 2026-07-06).
  `max_risk = |strike_a − strike_b| × 100 × qty`. Strikes only; the operator-typed net limit is NOT
  read for the risk number. Rationale: strongest fat-finger breaker (a mistyped credit — e.g. 0.60→6.0
  — cannot corrupt the risk), unifies credit & debit verticals under one formula, simplest to freeze.
  Rejected alternative (exact max-loss `credit: (width−credit)×100×qty` / `debit: debit×100×qty`): true
  risk but re-trusts the typed premium (a mistyped credit > width yields a negative "risk" that passes
  unconditionally) and needs clamp guards. Consequence, accepted by operator: the motivating NVDA
  185/180 (width $5 ⇒ risk exactly $500) sits AT the $500 default cap boundary — passes (`>` is the
  refuse, not `>=`; `check_live_write_posture` boundary is Ok), zero headroom; wider or cheap-debit
  verticals are conservatively over-counted and need a per-command `OMI_MAX_NOTIONAL` bump.

- **D3 — only a CLEAN 2-LEG 1:1 VERTICAL is unlocked on live; every other shape stays refused.**
  ✅ human-confirmed (/think) · 📖 predicate code-verified against `LegSpec` (`trade.rs:684`) +
  the same-underlying rule already enforced at `trade.rs:831`. A live combo is admitted iff ALL hold
  (else Err ⇒ refused, "live combo is limited to 2-leg vertical spreads; other structures are
  paper-only"):
  1. exactly 2 legs (`specs.len() == 2`) — else refused (calendars/butterflies/condors paper-only);
  2. same underlying (`specs[0].symbol == specs[1].symbol`) — already enforced upstream; re-assert in
     the pure seam so it is self-contained/freezable;
  3. same expiry (`specs[0].expiry == specs[1].expiry`) — else a calendar (diff expiry) ⇒ refused;
  4. same right (`specs[0].right == specs[1].right`, both `"C"` or both `"P"`) — else diagonal/straddle
     ⇒ refused;
  5. opposite actions (one `BUY`, one `SELL`) — else not a spread ⇒ refused;
  6. both `ratio == 1` — else a ratio/backspread (unbounded/undefined risk) ⇒ refused;
  7. distinct strikes (`specs[0].strike != specs[1].strike`) — else no width ⇒ refused.
  Keeps this a bounded, freezable geometry check, NOT the general risk engine ADR 0030 rejected.

- **D4 — same gate, same cap, no new bypass.** ✅ human-confirmed (/think) · 📖 `resolve_max_notional`
  (`trade.rs:208`) reused verbatim. Live combo still requires `--live` + `OMI_ALLOW_LIVE=1` (the
  unchanged `require_live_write_gate`). Cap = `OMI_MAX_NOTIONAL` (default `DEFAULT_MAX_NOTIONAL` $500),
  fail-closed on a bad value. The lockout reads no env today and no env escape hatch is added — the only
  way to place a live combo is this reviewed code change behind the gate.

- **D5 — cap compare reuses `check_live_write_posture`.** ⚠️ assumed (arch to confirm wording). Feed the
  computed `max_risk` as `check_live_write_posture(is_live=true, is_mkt=false, Some(max_risk), cap)` —
  reuses the frozen `> cap` refuse + `== cap` boundary semantics (combos are always LMT, so `is_mkt`
  is structurally false). Tradeoff arch decides: reuse ⇒ the over-cap message says "notional" not "max
  risk"; a thin combo-specific check gives a truer message but duplicates the compare. Not a blocker.

- **D6 — do NOT touch the ADR 0030 frozen spec.** ✅ human-confirmed (/think) · 📖 freeze-interaction
  verified: `tests/live_write_guardrail.rs` (spec-rev 817c7d8, feature DONE/merged) freezes
  `refuse_live_combo_on_live`. Deleting that fn breaks another card's frozen spec (AGENTS.md hard
  invariant: never edit another card's spec-paths). Therefore: **keep `refuse_live_combo_on_live`
  defined and re-exported (its frozen test stays green), only STOP CALLING it** — replace the
  `trade.rs:856` call site with the new vertical-posture logic. It stays `pub` + referenced by the
  frozen test ⇒ no `dead_code` warning. New behavior lives in a NEW frozen file
  `tests/live_combo_unlock.rs`; no re-freeze of any existing spec.

## Scope

- **IN** (`src/ib/trade.rs`): a NEW pure seam
  `combo_live_max_risk(specs: &[LegSpec], qty: f64) -> Result<f64, String>` — the D3 predicate + the D2
  width×100×qty computation (Ok(risk) for a clean vertical, Err(reason) otherwise). Rewire the
  `option_combo` `!cfg.preview` block (`trade.rs:852-857`): on `is_live` (`cfg.port == LIVE_PORT`),
  `combo_live_max_risk(&specs, args.qty).map_err(config)?` → `resolve_max_notional(env).map_err(config)?`
  → `check_live_write_posture(true, false, Some(risk), cap).map_err(config)?`, THEN the unchanged
  `require_live_write_gate(cfg)?`. Paper (`is_live == false`) skips the risk/cap block entirely.
- **IN** (`src/ib/mod.rs`): re-export `combo_live_max_risk` alongside the existing trade seams (`:45`).
- **IN** (`src/cli.rs`): help text only if the combo command help mentions the paper-only lockout.
- **IN**: NEW frozen spec `tests/live_combo_unlock.rs`.
- **OUT**: single-leg / STK path (`place_core` untouched); `require_live_write_gate`, `option-close`,
  `cancel`, `--preview`, paper behavior — all unchanged; combos with >2 legs or non-vertical shapes on
  live (stay refused); the exact-max-loss risk model (rejected, D2); a separate combo cap env (reuse
  `OMI_MAX_NOTIONAL`, D4); modify/GTC; the trade log (a separate future feature).

## Success criteria

1. `combo_live_max_risk` (offline, FROZEN) — clean put credit vertical `[SELL 1 NVDA 20260715 185 P,
   BUY 1 NVDA 20260715 180 P]`, qty 1 ⇒ `Ok(500.0)`; call vertical strikes 250/240 qty 2 ⇒ `Ok(2000.0)`;
   3 legs ⇒ Err; diff expiry ⇒ Err; diff right ⇒ Err; same action ⇒ Err; ratio 2 ⇒ Err; equal strikes ⇒
   Err; diff underlying ⇒ Err. [frozen]
2. `omi --live option-combo` for the NVDA 185/180 put credit (width $5 ⇒ risk $500, default cap $500) ⇒
   passes the guardrail and proceeds to the gate → connect (boundary `500 > 500` is false). With
   `OMI_ALLOW_LIVE=1` + `:4001` up it places; the actual place is operator live acceptance (would place
   a real order — NOT unit-tested), like every prior gateway-dependent behavior. [operator]
3. `omi --live option-combo` for a `185/175` @ any premium (width $10 ⇒ risk $1000 > $500) ⇒ refused,
   exit 5, "max risk … exceeds cap", BEFORE connect (works with `:4001` down). `OMI_MAX_NOTIONAL=1000`
   ⇒ passes. [offline logic + operator]
4. `omi --live option-combo` for a non-vertical (3 legs / calendar / straddle / ratio) ⇒ refused, exit
   5, "2-leg vertical spreads only", BEFORE connect. [offline logic]
5. Paper unaffected: `omi option-combo` (`:4002`) still places ANY valid combo shape (no risk/cap
   check); `omi --preview --live option-combo …` still previews (read-only, exempt). [operator paper]
6. Contained: diff = `src/ib/trade.rs` + `src/ib/mod.rs` (re-export) + optional `src/cli.rs` help + NEW
   `tests/live_combo_unlock.rs`. `tests/live_write_guardrail.rs` UNCHANGED and still green
   (`refuse_live_combo_on_live` defined + tested, unwired). `require_live_write_gate` / `place_core` /
   `option-close` / `cancel` untouched. [read / freeze gate]
7. `cargo build` · full `cargo test` · `cargo clippy --all-targets -- -D warnings` all green. [verify]

## Gotchas

- The whole risk/cap block fires ONLY on `cfg.port == LIVE_PORT && !cfg.preview`. Paper and preview are
  exempt by construction — mirror the ADR 0030 port key exactly.
- `combo_live_max_risk` inputs (`specs`, `args.qty`) are all available at `trade.rs:848` (after parse +
  qty validation, before connect) ⇒ the check is fully offline. Do NOT move it after `connect`.
- Do NOT delete `refuse_live_combo_on_live` (D6) — breaks `tests/live_write_guardrail.rs` freeze.
- qty is already validated whole ≥ 1 (`trade.rs:837`); the seam trusts that and just multiplies.
- Multiplier is 100 (option) — hardcode in the seam; combos are option-only.
- Do NOT run full `cargo test` while the Tiger gateway is UP unless the live-gate guard is on the branch
  (it is on trunk; branches cut from trunk inherit it).

## Verify

`cargo build` · `cargo test` · `cargo clippy --all-targets -- -D warnings` · operator paper: any combo
still places on `:4002`; offline refuse checks need NO gateway. Live-pass is the operator's first live
combo (the NVDA 185/180) post-merge, with `OMI_MAX_NOTIONAL` set for headroom if desired.
