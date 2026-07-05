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

## seq=2 · 2026-07-05T13:32:52Z · arch→task · completed · by=claude (cc)
done:   Architecture decision-complete. No arch-level human grill needed — the 3 PRD decisions are
        human-confirmed and every ⚠️assumed row settled by code (config/dispatch/ibapi-OrderState
        all read). Chosen shape: branch to a non-transmitting whatIf placement AT the placement
        call-site (AFTER require_live_write_gate) so the real transmit path stays byte-identical and
        the gate is reused unchanged (gate == real order by construction). Two new seams in
        src/ib/trade.rs: shape_preview(&Contract,&Order,&OrderState)->Value (pure FROZEN, mirrors the
        ack seam at trade.rs:51; OrderState derives Default+pub fields so it's a real-value literal,
        not a mock) and preview_with_client (gateway fn, review-by-reading). Flag plumbs
        GlobalOpts.preview→Config.preview; main.rs unchanged. ADR 0026 records it (0025 was taken by
        write-path-semantics). Tiger what_if premise tabled as R1/R2 risk register in CONTEXT.md —
        NOT frozen (live-acceptance only).
output: .pipeline/order-preview/arch.md, .pipeline/order-preview/CONTEXT.md,
        .pipeline/order-preview/docs/adr/0026-order-preview-whatif.md, .pipeline/current.json (stage=arch)
--- handoff ---
>>> NEXT
Run pipeline-task on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=feat/order-preview pr=none
Model: frontier SOTA required.
First: git pull --rebase; load repo config (.env if present, per CONTRACT step 2).
Read for context (before acting):
  - oh-my-ib/AGENTS.md + CLAUDE.md — repo conventions (write code ONLY in src/ib/trade.rs; no mocks). Read FIRST.
  - .pipeline/order-preview/PRD.md — what.
  - .pipeline/order-preview/arch.md — how: the branch shape, component boundaries, the two seams, the freeze boundary (task→gate).
  - .pipeline/order-preview/CONTEXT.md — glossary + R1/R2 reference-behavior risk register (Tiger what_if — NOT frozen).
  - .pipeline/order-preview/docs/adr/0026-order-preview-whatif.md — the binding decision.
Your task (concrete, numbered):
  1. Decompose into atomic landable card(s). Likely ONE card (cohesive: flag plumb + branch + shape_preview + wire all 6 verbs) — split only if a clean red-test boundary justifies it.
  2. Freeze the RED tests over spec-paths (ONE freeze commit, must compile + FAIL). Freeze EXACTLY the arch.md §Freeze boundary FROZEN set:
       a. `--preview` parses on all 6 verbs → dead-port connection envelope (black-box assert_cmd; mirror existing tests/ write-path parse tests).
       b. Config.preview merge unit (g.preview → cfg.preview).
       c. order.what_if==true after the preview transform; ==false on the real path.
       d. shape_preview exact envelope + OrderState→key mapping + Option::None→JSON null, built from real (Contract,Order) via the builders + a constructed OrderState{..Default::default()} literal.
  3. Do NOT freeze gateway behavior (R1/R2): that Tiger returns margin under what_if and does not transmit is review-by-reading + operator live-acceptance. Record this in the card's `## Freeze coverage` (frozen: flag/shape/what_if-flag; review-must-read: preview_with_client gateway wiring + no-transmit).
  4. Record spec-paths ∩ impl-paths = ∅; impl-paths = src/** (trade.rs branch + seams, cli.rs flag, config.rs field). Set current.json.full-verify = ["cargo build","cargo test"] (already set). Advance current.json.stage=task. Append journal seq=3.
Feature gotchas:
  - shape_preview is the ONLY meaningful FROZEN correctness surface (a pure crate exists via lib.rs, so tests/ CAN reach it — NOT a pure-binary case). Freeze it hard.
  - Keep the real transmit path byte-identical — a frozen test should assert what_if==false on a normal buy/sell so impl can't regress it.
  - ADR next free number was 0026 (0025 taken). Gate stays identical to a real order — do NOT freeze any test that implies preview is ungated/read-shaped.
  - Metadata + the red test commit straight to trunk (main); feat/order-preview is cut by impl.
Done when: cards + frozen RED tests exist (compile+fail); spec-rev recorded on every card; current.json.stage=task; journal seq=3. On success: run pipeline-impl (coder = π / GLM-5.2).
On failure: attempts++; >=3 ⇒ blocked ⇒ run pipeline-hunt.
<<< END
