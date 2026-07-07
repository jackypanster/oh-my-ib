# journal — outside-rth

## seq=1 · 2026-07-07T01:26:43Z · —→prd · completed · by=cc/claude-opus-4.8
done:   Grilled + locked the design via /think (2 operator decisions: single boolean flag; STK-only).
        Wrote PRD.md decision-complete. Code survey found the frozen-seam collision (build_stk_order
        arity pinned by 3 test files) and resolved it with a NEW seam apply_outside_rth (no arity change).
output: .pipeline/outside-rth/PRD.md
--- handoff ---
>>> NEXT
Run pipeline-arch on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
Model: frontier SOTA required — operator assigns the bot; the pipeline can't verify the model.
First: git pull --rebase; load repo config (.env if present, per CONTRACT step 2).
Read for context (before acting):
  - oh-my-ib/AGENTS.md + CLAUDE.md — repo conventions; ADR 0017 (write containment: writes ONLY in
    src/ib/trade.rs), ADR 0027 (read-only preview), ADR 0030/0031 (live gate + combo breaker). Read FIRST.
  - .pipeline/outside-rth/PRD.md — what/why + the 6 locked decisions (D1–D6) + gotchas.
  - src/ib/trade.rs (build_stk_order :31, shape_preview :79, place :603, preview_stk_option :515,
    place_core :568) — the STK write seams.
  - tests/{stk_orders_command,order_preview_command,write_path_semantics_doc}.rs — the frozen tests
    that pin build_stk_order (4-arg) and shape_preview (top-level keys). These MUST stay green.
Your task (concrete, numbered):
  1. Grill the architecture against the codebase (grill-with-docs). The design is decision-complete;
     your job is to pin the SEAM and confirm no ADR/invariant is violated, not re-open D1–D6.
  2. Confirm the new pure seam: `apply_outside_rth(order: &mut Order, outside_rth: bool)
     -> Result<(), String>` — MKT guard reads `order.order_type`; sets `order.outside_rth`. Pin the
     exact refuse message. Confirm it lives in src/ib/trade.rs (ADR 0017 containment) and is re-exported
     at src/ib/mod.rs:45.
  3. Confirm wiring: place() calls build_stk_order (UNCHANGED) then apply_outside_rth before place_core,
     Err→AppError::config (exit 5). Confirm shape_preview adds "outside_rth" INSIDE the "order" object.
  4. Decide + (if yes) author ADR 0032 recording: outside-RTH = single boolean (not 3 sessions),
     LMT-required guard, STK-only, guardrails orthogonal/untouched. Recommend a SHORT ADR for audit parity.
  5. Write arch.md + CONTEXT.md (glossary term "outside-RTH / extended hours = pre+post") + docs/adr/0032*.
     Advance current.json.stage=arch; append journal seq=2; commit metadata; push.
Feature gotchas (project-specific traps):
  - #1: do NOT change build_stk_order's signature — 3 frozen test files call it 4-arg; a 5th param
    breaks their compilation = editing another card's spec-paths (hard invariant). Use the new seam.
  - shape_preview: nest outside_rth INSIDE "order" (frozen test asserts only the 8 top-level keys).
  - Set the flag BEFORE place_core so both the --preview branch and the transmit branch carry it.
  - Guard is config/exit 5 (flag-combination), offline/pre-connect, ordered after usage < before gate.
  - STK-only: do NOT add the flag to option/combo/close verbs (no-op there, misleading).
  - Writes stay in src/ib/trade.rs only (ADR 0017); cli.rs only defines the arg.
Done when: arch.md + CONTEXT.md (+ ADR 0032 if chosen) landed, stage=arch, journal seq=2 appended.
On success: run pipeline-task (decompose into cards, freeze the red test tests/outside_rth.rs).
On failure: attempts++; >=3 ⇒ blocked ⇒ run pipeline-hunt.
<<< END
