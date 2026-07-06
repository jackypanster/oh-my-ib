# journal — live-combo-unlock

## seq=1 · 2026-07-06 · prd→arch · completed · by=claude-opus-4-8
done:   PRD written. Follows ADR 0030 (live-write-guardrail, merged #27/a843a08). ADR 0030 D4 hard-locks
        ALL live combos (`refuse_live_combo_on_live`, trade.rs:247, wired :856 before the gate) because
        the only breaker `compute_notional = qty×|limit|×100` reads the NET premium and under-counts a
        spread's real risk (NVDA 185/180 put credit @0.60: net-notional $60 vs true max loss $440; a
        185/175 @0.60 = $940 loss but still $60 net-notional ⇒ would pass a $500 cap). Operator is opening
        live combos ⇒ the ADR's "revisit only if live combo is opened" is due. Goal: REPLACE the lockout
        with a combo-defined-risk breaker, same gate (--live+OMI_ALLOW_LIVE=1), same cap
        (OMI_MAX_NOTIONAL, default $500). Decisions: D1 REPLACE not delete-and-fall-through (deleting =
        safety regression, codex #1 check); D2 risk = PURE WIDTH |Δstrike|×100×qty, premium-proof
        (operator-confirmed over exact-max-loss — width can't be corrupted by a mistyped credit; NVDA
        185/180 ⇒ $500, sits AT the $500 boundary, passes, zero headroom; bump OMI_MAX_NOTIONAL for
        room); D3 only a CLEAN 2-leg 1:1 vertical unlocked (2 legs, same underlying/expiry/right,
        opposite actions, ratio 1, distinct strikes — predicate code-verified vs LegSpec:684 +
        same-underlying already at :831), every other shape stays refused (NOT a risk engine); D4 same
        gate + same cap, no new env bypass; D5 reuse check_live_write_posture for the >cap/==cap compare
        (arch confirms message wording); D6 DO NOT delete/edit refuse_live_combo_on_live — it is frozen
        in tests/live_write_guardrail.rs (spec-rev 817c7d8, DONE feature); keep it defined+pub+unwired
        (no dead_code since pub + referenced by that frozen test), new behavior goes in a NEW frozen file
        tests/live_combo_unlock.rs, no re-freeze. Every refuse path is offline-deterministic (decided
        from specs+qty before connect) ⇒ freezable, like ADR 0030. Full-auto grant (per-feature):
        cc drives prd→arch→task, impl→OMP, review→codex, cc merges+deploys; stop at blocker/merge-confirm.
output: .pipeline/live-combo-unlock/PRD.md
--- handoff ---
>>> NEXT
Run pipeline-arch (cc, this feature). Assume the arch node knows nothing — rebuild from repo +
CONTRACT.md. repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
First: git pull --rebase; load repo config (CONTRACT step 2).
Read for context (before acting):
  - oh-my-ib/AGENTS.md + CLAUDE.md — agent-first; hard safety rules; Verify = 4 gates; freeze invariants.
  - .pipeline/live-combo-unlock/PRD.md — problem + D1-D6 + scope/non-scope + success criteria.
  - .pipeline/live-write-guardrail/docs/adr/0030-live-write-guardrail.md — the guardrail this extends;
    §Alternatives rejected "Notional = true risk … Revisit only if live combo is opened" = this feature.
  - src/ib/trade.rs — KEY seams: refuse_live_combo_on_live (247, KEEP — frozen by another card, only
    UNWIRE); check_live_write_posture (223, REUSE for the >cap/==cap compare); resolve_max_notional (208,
    REUSE, fail-closed); DEFAULT_MAX_NOTIONAL (195, $500); LegSpec (684, the risk-seam input); parse
    loop + same-underlying rule (821-836); qty validation (837); option_combo gate site (852-857, the
    rewire point); the preview arm (910+, unchanged).
  - src/ib/mod.rs:45 — trade re-export line (add combo_live_max_risk).
  - tests/live_write_guardrail.rs — the ADR 0030 frozen spec; MUST stay byte-identical + green.
Your task (concrete):
  1. Emit ADR 0031 (next number). Pin the pure seam signature
     `combo_live_max_risk(specs: &[LegSpec], qty: f64) -> Result<f64, String>` — the D3 clean-vertical
     predicate (7 conditions) + D2 width×100×qty; Err(reason) for every non-vertical.
  2. Pin the option_combo rewire: is_live gate → combo_live_max_risk → resolve_max_notional →
     check_live_write_posture(true,false,Some(risk),cap) → require_live_write_gate; paper skips; preview
     unchanged; map every Err via AppError::config (exit 5). Confirm D5 message wording (reuse vs thin
     combo check).
  3. Confirm D6: refuse_live_combo_on_live stays defined+re-exported, only unwired; no edit to
     tests/live_write_guardrail.rs.
  4. Freeze plan: NEW tests/live_combo_unlock.rs asserts combo_live_max_risk only (values + all Err
     shapes); wiring is review-by-reading; within-cap→place is operator live acceptance.
  5. Emit CONTEXT.md. Hand off to pipeline-task.
<<< END
