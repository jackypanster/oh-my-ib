# Journal — order-preview

## seq=1 · 2026-07-05T13:26:09Z · prd→arch · completed · by=claude (cc)
done:   PRD for the whatIf order-preview capability. /think-approved single highest-ROI todo:
        give the natural-language→hermes→live-money loop an "intent → preview → confirm → execute"
        step. 3 HITL decisions confirmed by operator: (1) surface = global `--preview` flag;
        (2) scope = all six order verbs; (3) gate = identical to a real order (fail-safe vs Tiger
        ignoring what_if). Design mirrors the existing pure-FROZEN ack seam (trade.rs:51) + treats
        OrderState margin/commission (all Option<f64>) extraction as review-by-reading; ships value
        even if Tiger ignores what_if (echo + resolved contract always present, margin null-if-absent).
output: .pipeline/order-preview/PRD.md, .pipeline/current.json (stage=prd)
--- handoff ---
>>> NEXT
Run pipeline-arch on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=feat/order-preview pr=none
Model: frontier SOTA required.
First: git pull --rebase; load repo config (.env if present, per CONTRACT step 2).
Read for context (before acting):
  - oh-my-ib/AGENTS.md + CLAUDE.md — repo conventions (agent-first; writes gated; write code ONLY in src/ib/trade.rs). Read FIRST.
  - .pipeline/order-preview/PRD.md — what: whatIf `--preview` for the 6 order verbs, uniform envelope, same gate as a real order.
  - docs/write-path-semantics.md — the shipped write-path field-semantics audit (what_if:false is a load-bearing default row; transmit:true). Preview flips what_if→true for preview only.
Your task (concrete, numbered):
  1. Read src/ib/trade.rs — the choke point place_with_client (trade.rs:317: place_order + first OpenOrder/OrderState ack), place_core (:381), the pure FROZEN ack seam (:51), require_live_write_gate (:143), and the builders build_stk_order/build_option_order/build_combo_order.
  2. Read src/cli.rs GlobalOpts (:20-43) + src/config.rs Config (:38) — decide how `--preview` flows GlobalOpts.preview → Config.preview (mirror how --live collapses into Config.port).
  3. Decide the place_with_client preview branch: a `what_if: bool` param vs a sibling preview_with_client — CONSTRAINT: the real transmit path (what_if=false) must stay byte-identical so the frozen stk/option/combo/close suites stay green.
  4. Fix the final uniform envelope key set (PRD §Decisions 5 is the ⚠️assumed default — refine within "uniform + stable across all 6 verbs"). Sources are code-verified ibapi OrderState fields: initial/maintenance_margin_change, equity_with_loan_change, commission/minimum_commission/maximum_commission, commission_currency, warning_text, status.
  5. Fix where the pure `shape_preview(...)` FROZEN seam lives and the review-by-reading boundary (OrderState→envelope extraction is gateway, NOT frozen — same class as trade.rs:259 gateway fns).
  6. Emit arch.md + CONTEXT.md (+ ADR if a decision is binding, e.g. the gate-stays-same and the preview-branch shape). Advance current.json.stage=arch. Append journal seq=2. Do NOT touch src/ or tests/.
Feature gotchas (project-specific traps):
  - Write code lives ONLY in src/ib/trade.rs (AGENTS.md hard rule) — the preview branch + shape_preview go there.
  - Tiger-what_if premise (PRD §Risk D1): if Tiger ignores what_if, "preview" transmits a REAL order. Do NOT design any relaxation of the gate; keep it identical to a real order in v1. Read-shaped relaxation is explicitly deferred + evidence-gated.
  - Gateway fns are review-by-reading (NOT frozen — need a live gateway); frozen tests can only reach pure builder output + JSON shape + CLI parse. No mocks (repo hard rule).
  - Metadata commits straight to trunk (main); the feat/order-preview branch is cut later by impl.
Done when: arch.md + CONTEXT.md exist, decision-complete; current.json.stage=arch; journal seq=2 appended. On success: run pipeline-task.
On failure: attempts++; >=3 ⇒ blocked ⇒ run pipeline-hunt.
<<< END
