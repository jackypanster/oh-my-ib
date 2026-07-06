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

## seq=3 · 2026-07-06 · task→impl · completed · by=claude-opus-4-8
done:   FROZEN spec tests/live_combo_unlock.rs (spec-rev e2f2b171c01ca992829243c4b241e75f21847856) —
        combo_live_max_risk: 3 Ok (put credit / call qty2 / debit vertical; debit==credit⇒500 proves
        premium-proof) + 8 Err (1 leg, 3 legs, diff expiry, diff right, same action, ratio≠1, equal
        strikes, diff underlying). RED = unresolved import oh_my_ib::ib::combo_live_max_risk. Ran the
        MANDATORY verbatim-compile pre-check: scratch-stubbed the seam in trade.rs + mod.rs re-export,
        `cargo test --test live_combo_unlock --no-run` compiled clean AND `cargo clippy --all-targets -D
        warnings` clean (module-doc written as plain prose, no bullet-before-prose ⇒ no
        doc_lazy_continuation; float asserts via assert_eq! like live_write_guardrail.rs ⇒ no float_cmp),
        then `git checkout src/` reverted the stub — freeze commit contains ONLY the test. Card 01:
        impl-paths = src/ib/trade.rs + src/ib/mod.rs (cli.rs excluded — combo help has no "paper-only"
        text); spec-paths = tests/live_combo_unlock.rs ONLY; card carries a clippy-clean reference impl +
        the float_cmp gotcha (strike-equality via `width == 0.0`, comparison-to-zero is float_cmp-exempt;
        direct a.strike==b.strike would trip -D warnings) + the KEEP-but-UNWIRE rule for
        refuse_live_combo_on_live (D6). full-verify now includes clippy. Two-commit freeze protocol
        honored (freeze commit = test only; this record commit = card + current.json + journal).
output: tests/live_combo_unlock.rs (spec-rev e2f2b17) · .pipeline/live-combo-unlock/tasks/01.md
--- handoff ---
>>> NEXT
Run pipeline-impl on OMP (impl slot = goal-driven-impl-claude). repo=git@github.com:jackypanster/oh-my-ib.git
branch=main → cut feat/live-combo-unlock. pr=none. card=01.
First: git pull --rebase; branch feat/live-combo-unlock off main.
Read (before coding): .pipeline/live-combo-unlock/tasks/01.md (the card — §Scope is verbatim-enough to
implement; §Out-of-scope is load-bearing), arch.md §Chosen-shape + §Write-set, ADR 0031 §Seams, AGENTS.md
(agent-first; write code ONLY in src/ib/trade.rs for the write path; 4 verify gates).
Do (goal = make the frozen RED green, editing ONLY impl-paths):
  1. Add `pub fn combo_live_max_risk(specs: &[LegSpec], qty: f64) -> Result<f64, String>` to
     src/ib/trade.rs (7-condition clean-vertical predicate + width×100×qty; reference body in the card;
     compare strikes via `width == 0.0`, NOT `a.strike == b.strike`).
  2. Re-export it at src/ib/mod.rs:45.
  3. Rewire option_combo: REPLACE the single `refuse_live_combo_on_live(...)` call (inside `if
     !cfg.preview`, before `require_live_write_gate`) with the is_live→risk→cap→check_live_write_posture
     block from the card. KEEP refuse_live_combo_on_live DEFINED + re-exported (do NOT delete it).
  4. Do NOT touch tests/ (both live_combo_unlock.rs and live_write_guardrail.rs stay byte-identical).
Verify before PR: `cargo build` · `cargo test --test live_combo_unlock` GREEN · full `cargo test`
(live_write_guardrail still GREEN) · `cargo clippy --all-targets -- -D warnings` clean. Open PR from
feat/live-combo-unlock. Then hand to pipeline-review on codex (writer≠reviewer); cc merges + deploys.
<<< END

## seq=4 · 2026-07-06 · review→done · completed · by=claude-opus-4-8
done:   MERGED. PR #28 squash-merged to main (6de5354) after operator confirm (gated on CodeRabbit
        SUCCESS). Writer=OMP(π,GLM-5.2), reviewer=codex(gpt-5.5) — writer≠reviewer. codex ACCEPT, no
        findings: verified the live combo path feeds width-based combo_live_max_risk(&specs,args.qty)
        into the cap check (NOT net-premium — the #1 regression check); refuse_live_combo_on_live
        kept+re-exported+unwired; both frozen specs byte-identical (git diff --exit-code clean); diff =
        the two impl-path files only; self-ran build/test/clippy green. cc gates: freeze diff
        e2f2b17..branch over tests/ EMPTY; containment = trade.rs + mod.rs (+43/-5), refuse_live_combo_
        on_live defined but 0 callers in src; CodeRabbit pass; mergeStateStatus CLEAN. Trunk full-verify
        post-merge (6de5354): cargo build OK, full cargo test all green (live_combo_unlock 11/11,
        live_write_guardrail still green), clippy --all-targets -D warnings clean. OMP opened the PR
        itself (poll missed it by timing); cc gate-checked the branch before dispatching codex. Card 01
        done. FEATURE DONE — live combo (clean 2-leg 1:1 vertical) UNLOCKED under the ADR 0030 gate + cap.
output: main 6de5354 (#28); .pipeline/live-combo-unlock/reviews/review-01.md
--- handoff ---
>>> NEXT
Feature live-combo-unlock is DONE (current.json stage=done). No feature in flight. The ADR 0030 combo
lockout is replaced by a pure-width defined-risk breaker: `omi --live option-combo` now admits a clean
2-leg 1:1 vertical whose max risk (|Δstrike|×100×qty) is ≤ OMI_MAX_NOTIONAL (default $500), gated by
--live + OMI_ALLOW_LIVE=1; every other combo shape refuses exit 5 before connect; paper/preview/
single-leg/close/cancel unchanged.
OPERATOR LIVE ACCEPTANCE (the trial): flip Tiger to :4001; the first live combo is the NVDA 20260715
185/180 put credit (width $5 ⇒ risk $500 == default cap ⇒ passes; set OMI_MAX_NOTIONAL=600 for headroom).
Over-cap (185/175) and non-vertical live combos must refuse exit 5 with no order (verifiable :4001 down).
See reviews/review-01.md §OPERATOR LIVE ACCEPTANCE. NEXT feature (operator-decided): the append-only
trade log (~/.local/state/oh-my-ib/trades.jsonl, repo-external) — start with /pipeline-prd when ready.
<<< END

## seq=5 · 2026-07-06 · live-acceptance → broker-blocked · by=claude-opus-4-8
done:   Operator-authorized first live combo on :4001 (account U20230856). Preview-confirmed the BAG
        (--live --preview, transmits:false): SELL 185P conid 897191251 / BUY 180P conid 897191240, limit
        -0.6 credit, qty 1, net-premium notional $60. Real placement REJECTED by the gateway:
        {"error":{"code":"data","context":"option-combo","message":"order stream: [460] No trading
        permissions."}}. Read: code=data (order-stream/broker-side, NOT code=config/exit5) ⇒ the order
        PASSED the ADR 0031 width breaker ($500==cap⇒allowed) + live gate + connected to :4001 and reached
        the broker — the guardrail chain is END-TO-END VALIDATED. Broker refused: account lacks
        options/spread entitlement on Tiger. NO order placed (omi --live orders shows only the pre-existing
        BUY 1 NVDA LMT 195, order_id 2). BLOCKER = Tiger account permission, not a code fix. Re-run the
        identical command once options/combo trading is enabled. Recorded in reviews/review-01.md
        §LIVE-ACCEPTANCE RESULT + memory project_live-combo-unlock-acceptance-pending.
output: reviews/review-01.md (live-acceptance result appended)
<<< END

## seq=2 · 2026-07-06 · arch→task · completed · by=claude-opus-4-8
done:   ADR 0031 emitted (docs/adr/0031-live-combo-unlock.md) + CONTEXT.md + arch.md. Code-first verified
        every PRD claim: LegSpec fields (trade.rs:684); specs OWNED at the :852-856 rewire point (into_iter
        consume at :872, after) ⇒ &specs borrow valid; check_live_write_posture over-cap msg = "live
        notional {n} exceeds cap {cap}" (:236); sole production caller of refuse_live_combo_on_live is :856
        (mod.rs:45 re-export + tests/live_write_guardrail.rs:120 test) ⇒ unwiring = delete that ONE line,
        fn stays pub+tested ⇒ no dead_code; ADR next number 0031. Pinned: (1) NEW pure seam
        `combo_live_max_risk(specs:&[LegSpec], qty:f64) -> Result<f64,String>` = D3 7-condition
        clean-vertical predicate + D2 width×100×qty (premium-proof — net limit NOT a param); (2)
        option_combo rewire: replace :856 lockout line with `is_live` block → combo_live_max_risk →
        resolve_max_notional(env) → check_live_write_posture(true,false,Some(risk),cap), all Err→
        AppError::config(exit5), THEN unchanged require_live_write_gate; paper/preview skip. (3) D5
        RESOLVED: reuse check_live_write_posture verbatim; over-cap msg keeps "notional" wording (it IS the
        risk-notional; CC's action unchanged) ⇒ combo_live_max_risk is the ONLY new frozen surface. (4) D6
        RESOLVED: refuse_live_combo_on_live stays defined+re-exported+frozen, only unwired;
        tests/live_write_guardrail.rs byte-identical. (5) NEW deliberate decision: posture-BEFORE-gate for
        combo (differs from place_core gate-first) — a shape/cap refusal holds regardless of
        OMI_ALLOW_LIVE, so it must report before the env gate (else misdirect). Reference-behavior
        artifact: N/A (pure arithmetic over already-parsed LegSpec; no new external dependency). No
        blocking ambiguity ⇒ no human grill needed.
output: .pipeline/live-combo-unlock/docs/adr/0031-live-combo-unlock.md · CONTEXT.md · arch.md
--- handoff ---
>>> NEXT
Run pipeline-task (cc, this feature). repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
First: git pull --rebase.
Read: .pipeline/live-combo-unlock/arch.md (§Freeze plan = the exact assertions) + ADR 0031 §Seams &
§Freeze-coverage + CONTEXT.md + src/ib/trade.rs (LegSpec:684, combo_live_max_risk does not exist yet,
option_combo:799) + tests/live_write_guardrail.rs (the frozen-spec authoring style to mirror; leave it
untouched).
Your task (single atomic card, single spec-rev):
  1. Author NEW frozen spec tests/live_combo_unlock.rs — RED (import `oh_my_ib::ib::combo_live_max_risk`,
     which does not resolve yet). Assert ONLY combo_live_max_risk per arch.md §Freeze plan:
     Ok — put credit vertical (SELL 185P/BUY 180P qty1)⇒500.0; call vertical 250/240 qty2⇒2000.0; debit
     vertical (BUY 180P/SELL 185P qty1)⇒500.0 (premium-proof: same width as credit). Err (8, each a
     distinct reason) — 1 leg; 3 legs; diff expiry; diff right; same action; ratio 2; equal strikes; diff
     underlying. Construct LegSpec literals (pub fields).
  2. PRE-FREEZE CLIPPY GATE (learned rule): before freezing, stub the seam in a scratch build and run
     `cargo clippy --all-targets -- -D warnings` on the test file so the frozen spec is clippy-clean —
     a clippy-dirty frozen test would corner the coder into a freeze violation.
  3. impl-paths for card 01 = src/ib/trade.rs + src/ib/mod.rs (+ src/cli.rs help if present). spec-paths
     = tests/live_combo_unlock.rs ONLY. Do NOT list tests/live_write_guardrail.rs anywhere.
  4. Record card 01 (frozen spec-rev), set current.json stage=task, journal the task→impl handoff to OMP.
Handoff target: pipeline-impl on OMP (Claude-native goal-driven-impl) — make the RED green by editing
ONLY impl-paths; codex reviews; cc merges.
<<< END
