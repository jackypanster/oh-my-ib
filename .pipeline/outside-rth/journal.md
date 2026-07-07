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

## seq=2 · 2026-07-07T01:30:38Z · prd→arch · completed · by=cc/claude-opus-4.8
done:   grill-with-docs confirmed the seam against the codebase (no ADR/invariant violated). Wrote
        arch.md (chosen shape + write-set + freeze plan), CONTEXT.md (glossary), ADR 0032 (accepted).
        Locked the new pure seam apply_outside_rth (post-build; MKT-guard; config/exit5) — NO
        build_stk_order arity change; preview echo nested inside "order" (non-breaking to frozen test).
output: .pipeline/outside-rth/arch.md, .pipeline/outside-rth/CONTEXT.md, .pipeline/outside-rth/docs/adr/0032-outside-rth.md
--- handoff ---
>>> NEXT
Run pipeline-task on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
Model: frontier SOTA required — operator assigns the bot; the pipeline can't verify the model.
First: git pull --rebase; load repo config (.env if present, per CONTRACT step 2).
Read for context (before acting):
  - oh-my-ib/AGENTS.md + CLAUDE.md — repo conventions (write containment ADR 0017: writes ONLY in
    src/ib/trade.rs). Read FIRST.
  - .pipeline/outside-rth/PRD.md — what/why + locked decisions D1–D6.
  - .pipeline/outside-rth/arch.md — the chosen shape, the exact write-set, and the FREEZE PLAN (this is
    your spec: what tests/outside_rth.rs must assert).
  - .pipeline/outside-rth/CONTEXT.md — glossary. docs/adr/0032-outside-rth.md — binding decisions.
  - tests/order_preview_command.rs — MIRROR its black-box style (omi(), expect_error_code) + its
    top-level-keys assertion pattern for the preview test. tests/stk_orders_command.rs — build_stk_order
    import + LegSpec/Order literal style.
Your task (concrete, numbered):
  1. Decompose: this is small enough for ONE card (01) — the apply_outside_rth seam + preview echo + CLI
     guard are one coherent unit. Do NOT over-split.
  2. Write the FROZEN red test tests/outside_rth.rs per arch.md §Freeze plan: (a) seam — LMT+true⇒Ok∧true,
     LMT+false⇒Ok∧false, MKT+true⇒Err(contains "limit"), MKT+false⇒Ok∧false; (b) preview echo —
     out["order"]["outside_rth"] true/false; (c) CLI black-box — buy --outside-rth no --limit ⇒ config
     (exit 5), buy --limit 1 --outside-rth --port 65000 ⇒ connection. Import
     oh_my_ib::ib::{build_stk_order, apply_outside_rth, shape_preview}.
  3. The test MUST COMPILE and FAIL now (apply_outside_rth does not exist yet + shape_preview lacks the
     key). Confirm: compile the test as a stub if needed, then `cargo test --test outside_rth` is RED.
     BEFORE freezing, verify the stub compiles clean under `cargo clippy --all-targets -D warnings`
     (SKILL-PROPOSAL trap from live-gate-test-safety — dodge it manually: a red test that doesn't
     compile-clean blocks impl).
  4. Freeze commit (ONLY tests/outside_rth.rs) → its sha = the feature spec-rev. Record commit: write
     tasks/01.md frontmatter (status: todo, attempts: 0, spec-rev=<sha>, spec-paths: [tests/outside_rth.rs],
     impl-paths: [src/cli.rs, src/ib/trade.rs, src/ib/mod.rs], verify: `cargo test --test outside_rth`).
     Assert spec-paths ∩ impl-paths = ∅. Set current.json.full-verify already = [cargo build, cargo test,
     cargo clippy --all-targets -- -D warnings] (correct as-is). Advance stage=task; append journal seq=3.
  5. Commit metadata (two-commit protocol: freeze commit FIRST, then record commit); push.
Feature gotchas (project-specific traps):
  - #1: do NOT change build_stk_order's signature (3 frozen files call it 4-arg). The card's impl adds a
    NEW seam apply_outside_rth. spec-paths is ONLY tests/outside_rth.rs.
  - shape_preview: the preview test asserts the NESTED out["order"]["outside_rth"], NOT a new top-level key
    (existing order_preview_command.rs still owns the top-level assertion and must stay green).
  - Guard bucket is config/exit 5 (flag combination), offline. The CLI black-box test asserts code="config".
  - card verify is CARD-SCOPED (`cargo test --test outside_rth`), never the full suite (trunk is red until
    impl merges). full-verify (whole suite) is review's gate.
  - Do NOT run full `cargo test` while the Tiger gateway is UP unless the live-gate guard is on the branch
    (it is on trunk; branches inherit it).
Done when: tests/outside_rth.rs frozen+RED, tasks/01.md recorded, stage=task, journal seq=3.
On success: hand impl to omp (π) — run pipeline-impl. On failure: attempts++; >=3 ⇒ blocked ⇒ pipeline-hunt.
<<< END
