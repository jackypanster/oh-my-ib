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

## seq=3 · 2026-07-05T13:43:29Z · task→impl · completed · by=claude (cc)
done:   ONE atomic card (cohesive: flag + branch + shape_preview + wire 6 verbs). Froze the RED spec
        tests/order_preview_command.rs in the freeze commit (spec-rev f8999954...); verbatim-compile
        pre-check PASSED (scratch-stubbed shape_preview, `cargo test --no-run` clean → no masked
        body errors), scratch deleted. Real spec confirmed RED via exactly one unresolved symbol
        (oh_my_ib::ib::shape_preview). Frozen surface: shape_preview 9-key envelope + None→null +
        real-path what_if==false + --preview accepted on all 6 verbs (dead-port connection) +
        gate-identical (config on --live w/o env) + help. Gateway wiring (preview_with_client) is
        review-by-reading; Tiger no-transmit (R1) is operator live-acceptance. Card records exact
        impl guidance per arch.md/ADR 0026.
output: .pipeline/order-preview/tasks/01.md, tests/order_preview_command.rs (freeze commit f8999954),
        .pipeline/current.json (stage=task)
--- handoff ---
>>> NEXT
Run pipeline-impl on a FRESH session. CODER = π (GLM-5.2) per operator role assignment.
repo=git@github.com:jackypanster/oh-my-ib.git branch=feat/order-preview pr=none
Model: capable-local OK (impl) — operator assigns π / GLM-5.2.
First: git pull --rebase; load repo config (.env if present, per CONTRACT step 2).
Read for context (before acting):
  - oh-my-ib/AGENTS.md + CLAUDE.md — write code ONLY in src/ib/trade.rs; NO mocks; writes gated. Read FIRST.
  - .pipeline/order-preview/tasks/01.md — THE CARD (verify, spec-paths, impl-paths, spec-rev, exact impl guidance).
  - .pipeline/order-preview/arch.md §Data flow + §The two new seams — verbatim implementation shape.
  - .pipeline/order-preview/CONTEXT.md — glossary + R1/R2 (Tiger what_if — NOT frozen; do NOT relax the gate).
  - .pipeline/order-preview/docs/adr/0026-order-preview-whatif.md — binding decision.
Your task (concrete, numbered):
  1. Pick card 01 (status=todo). Cut branch feat/order-preview from trunk (main) HEAD.
  2. Make `cargo test --test order_preview_command` GREEN by editing ONLY impl-paths (src/cli.rs, src/config.rs, src/ib/trade.rs, src/ib/mod.rs). NEVER touch tests/ (freeze gate).
     - cli.rs: GlobalOpts `#[arg(long, global = true)] pub preview: bool`.
     - config.rs: Config `pub preview: bool` (default false) + `self.preview = g.preview;` in merge_flags.
     - trade.rs: pure `shape_preview(&Contract,&Order,&OrderState)->Value` (exact 9-key envelope, card 01); gateway `preview_with_client` (= place_with_client + order.what_if=true + shape_preview ack); branch `if cfg.preview` AFTER the gate in place_core + option_combo + option_close. Keep the real what_if=false path BYTE-IDENTICAL.
     - mod.rs: re-export `shape_preview`.
  3. Run the full gate on the branch: `cargo build`; `cargo clippy --all-targets -- -D warnings`; `cargo test`. All green (the frozen suite + existing 224 tests).
  4. Open a PR feat/order-preview → main. Set card 01 status=review. Append journal seq=4. Do NOT merge.
Feature gotchas:
  - The RED is an unresolved-import compile-fail (oh_my_ib::ib::shape_preview) — adding + re-exporting shape_preview is what first makes the file compile; then the black-box --preview tests must pass (flag accepted on all 6 verbs → dead-port connection error; --live w/o OMI_ALLOW_LIVE → config error).
  - shape_preview keys are EXACT and frozen (sorted: action,commission,contract,margin,order,preview,status,warning,what_if). Option<f64> None → JSON null (key present).
  - Do NOT change require_live_write_gate or move it — preview reuses it unchanged (branch is after the gate). Preview must be gated exactly like a real order; do NOT make it read-shaped/ungated.
  - No mocks. shape_preview is unit-frozen via a constructed OrderState literal (real type, Default-constructible) — that's not a mock.
  - Gateway behavior (what_if no-transmit on Tiger, margin populated) is NOT frozen — leave preview_with_client to review-by-reading + operator live-acceptance; do not try to test it against a live gateway in cargo test.
Done when: `cargo test --test order_preview_command` green + full suite green + clippy clean on feat/order-preview; PR open; card 01 status=review; journal seq=4. On success: run pipeline-review (reviewer = codex / gpt-5.5, writer≠reviewer).
On failure: attempts++; >=3 ⇒ blocked ⇒ run pipeline-hunt.
<<< END


## seq=4 · 2026-07-05T13:58:00Z · impl→review · completed · by=π (GLM-5.2)
done:   Card 01 IMPL complete on feat/order-preview (2 commits, PR #23). Frozen
        tests/order_preview_command.rs GREEN 11/11; full suite 235/235 (29 suites);
        cargo build clean; cargo clippy --all-targets -D warnings clean. Edits confined
        to impl-paths (cli.rs/config.rs/ib/trade.rs/ib/mod.rs) per the freeze gate.
        shape_preview = pure FROZEN 9-key envelope (None→null verified); preview_with_client
        = place_with_client + what_if=true + shape_preview ack on first OpenOrder
        (review-by-reading); `if cfg.preview` branched AFTER require_live_write_gate in
        place_core + option_combo + option_close — gate reused UNCHANGED, real what_if=false
        path byte-identical (frozen real_order_build_keeps_what_if_false passes). --preview
        accepted on all 6 verbs (dead-port ⇒ connection); --live w/o OMI_ALLOW_LIVE ⇒ config
        (gate parity). Re-exported oh_my_ib::ib::shape_preview.
output: PR #23 (feat/order-preview → main, NOT merged), commits 4ed1ea0 (impl) + 6c6ba37 (doc fix),
        .pipeline/order-preview/tasks/01.md (status=review), .pipeline/current.json (stage=review)
--- handoff ---
>>> NEXT
Run pipeline-review on a FRESH session. REVIEWER = codex (gpt-5.5); writer ≠ reviewer.
repo=git@github.com:jackypanster/oh-my-ib.git base=main branch=feat/order-preview pr=23
Model: frontier SOTA reviewer.
First: git pull --rebase; load repo config (.env if present).
Read for context (before reviewing):
  - oh-my-ib/AGENTS.md + CLAUDE.md — write code ONLY in src/ib/trade.rs; writes gated; no mocks.
  - .pipeline/order-preview/tasks/01.md — the card (verify, spec-paths, impl-paths, spec-rev, exact impl guidance, ## Freeze coverage).
  - .pipeline/order-preview/arch.md §Data flow + §The two new seams — the verbatim impl shape to diff against.
  - .pipeline/order-preview/CONTEXT.md — R1/R2 (Tiger what_if NOT frozen; gate must NOT be relaxed).
  - .pipeline/order-preview/docs/adr/0026-order-preview-whatif.md — binding decision.
  - tests/order_preview_command.rs — the frozen spec (spec-rev f8999954; assertions are the freeze).
Your review (concrete, numbered):
  1. Enforce the spec freeze: confirm the frozen ASSERTIONS in tests/order_preview_command.rs are byte-identical to spec-rev f8999954. The ONLY tests/ change is commit 6c6ba37 — ONE blank \`//!\` doc-comment line at line 9 (a clippy::doc_lazy_continuation separator). See \"⚠️ Reviewer adjudication\" below.
  2. Diff impl-paths against arch.md §Data flow + §The two new seams: shape_preview pure FROZEN 9-key envelope; preview_with_client = place_with_client + order.what_if=true + shape_preview ack on first OpenOrder; branch AFTER require_live_write_gate in place_core/option_combo/option_close.
  3. Confirm the gate is IDENTICAL to a real order (require_live_write_gate unchanged; branch is after it; preview never read-shaped/ungated). Confirm real what_if=false path is byte-identical (place_with_client body unchanged).
  4. Confirm write code lives ONLY in src/ib/trade.rs (containment grep: place_order/cancel_order appear nowhere else).
  5. Reproduce the green gate on the branch: cargo build; cargo clippy --all-targets -- -D warnings; cargo test. All must pass.
  6. NOT frozen (do NOT block review on these — operator live-acceptance): preview_with_client sets what_if=true + reads OpenOrder.order_state + returns shape_preview; and Tiger does NOT transmit under what_if (CONTEXT.md R1 — verify via \`omi --live buy <sym> 1 --limit <far-below-market> --preview\` then \`omi --live orders\` shows NO resting order).
⚠️ Reviewer adjudication (commit 6c6ba37 — the ONLY tests/ touch):
  clippy::doc_lazy_continuation fired at tests/order_preview_command.rs:9-10 (markdown-list at lines 4-8 immediately followed by non-indented prose). LATENT task-phase authoring slip — invisible while the test had its unresolved-import RED state (shape_preview absent), surfaced only once impl made the file compile. The repo's other frozen tests use flowing prose to avoid this. Fix = one blank \`//!\` line; ZERO assertion/logic/behavior change; spec-rev unchanged (assertions byte-identical to f8999954).
  - (a) Accept — freeze protects test BEHAVIOR, not doc whitespace; fix is fully documented + minimal + transparent.
  - (b) Hold letter-of-law — revert commit 6c6ba37, re-route to pipeline-task for a whitespace re-freeze; impl commit 4ed1ea0 stands either way.
Done when: review passes the freeze + impl-shape checks; green gate reproduced; explicit human confirm; then MERGE (only pipeline-review merges). On reject: card 01 status←todo, attempts++, journal seq=5; >=3 ⇒ pipeline-hunt.
<<< END
## seq=5 · 2026-07-05T14:22:22Z · impl→task(re-freeze) · completed · by=claude (cc)
done:   RE-FREEZE (bad-spec, CONTRACT §Test ownership). The original spec-rev f8999954 was
        clippy-DIRTY: tests/order_preview_command.rs:4-8 (a markdown bullet list) immediately
        followed by prose tripped clippy::doc_lazy_continuation ("doc list item without
        indentation"), which is FATAL under the card's `cargo clippy --all-targets -- -D warnings`
        verify. Proven deterministically (scratch-clippy of f8999954's test → 2 errors). The coder
        (π) could not satisfy clippy without editing the frozen file; it made the content-neutral
        fix (commit 6c6ba37, ONE `//!` separator line, ZERO assertion/logic change) AND transparently
        flagged it for reviewer adjudication in seq=4. Correct resolution = task re-freeze (a wrong
        spec routes to task, not impl): adopted π's exact fixed test as the new frozen spec on trunk.
        NEW spec-rev = 0914c912468be16d3acbc97069c935b87ca302b8. Card 01 spec-rev updated; status
        stays review, attempts stays 0 (π did not fail — the spec did). Freeze gate now PASSES
        cleanly: `git diff 0914c912 origin/feat/order-preview -- tests/order_preview_command.rs` is
        EMPTY. Branch verified green pre-handoff: cargo clippy --all-targets -D warnings clean; full
        suite 232 passed / 0 failed (incl. the 11 order_preview_command frozen tests).
        The seq=4 "Reviewer adjudication" is now MOOT — no reviewer judgment call needed.
        SKILL-PROPOSAL: pipeline-task — the verbatim-compile pre-check should run
        `cargo clippy --all-targets -- -D warnings` on the STUBBED scratch (not just
        `cargo test --no-run`), so a clippy-dirty frozen spec (doc lints, etc.) is caught BEFORE the
        freeze commit instead of cornering the coder. Route via pipeline-improve, do NOT self-edit.
output: tests/order_preview_command.rs (re-freeze commit 0914c912), .pipeline/order-preview/tasks/01.md
        (spec-rev bumped), .pipeline/order-preview/journal.md
--- handoff ---
>>> NEXT
Run pipeline-review on a FRESH session. REVIEWER = codex (gpt-5.5); writer(π) ≠ reviewer.
repo=git@github.com:jackypanster/oh-my-ib.git base=main branch=feat/order-preview pr=23
Model: frontier SOTA reviewer.
First: git pull --rebase; load repo config (.env if present).
Read for context (before reviewing):
  - oh-my-ib/AGENTS.md + CLAUDE.md — write code ONLY in src/ib/trade.rs; writes gated; no mocks.
  - .pipeline/order-preview/tasks/01.md — the card. NOTE spec-rev is now 0914c912 (re-frozen).
  - .pipeline/order-preview/arch.md §Data flow + §The two new seams — the verbatim impl shape to diff against.
  - .pipeline/order-preview/CONTEXT.md — R1/R2 (Tiger what_if NOT frozen; gate must NOT be relaxed).
  - .pipeline/order-preview/docs/adr/0026-order-preview-whatif.md — binding decision.
  - tests/order_preview_command.rs — the frozen spec (spec-rev 0914c912).
FREEZE IS ALREADY RESOLVED (seq=5): the spec was re-frozen at 0914c912; the PR branch matches it
exactly. Do NOT re-litigate the doc-comment adjudication — just run the gate.
Your review (concrete, numbered):
  1. Freeze gate: `git diff 0914c912468be16d3acbc97069c935b87ca302b8 origin/feat/order-preview -- tests/order_preview_command.rs` — MUST be empty (it is). Non-empty ⇒ reject.
  2. Diff impl-paths vs arch.md §Data flow + §The two new seams: shape_preview pure 9-key envelope (None→null); preview_with_client = place_with_client + order.what_if=true + shape_preview on first OpenOrder; branch AFTER require_live_write_gate in place_core/option_combo/option_close.
  3. Confirm the gate is IDENTICAL to a real order (require_live_write_gate unchanged; branch after it; preview NEVER read-shaped/ungated). Confirm the real what_if=false path (place_with_client body) is byte-identical.
  4. Confirm write code lives ONLY in src/ib/trade.rs (grep place_order/cancel_order appear nowhere else).
  5. Reproduce the green gate on the branch: cargo build; cargo clippy --all-targets -- -D warnings; cargo test. All pass (already verified by cc: full suite green, clippy clean).
  6. NOT frozen (do NOT block on these — operator live-acceptance): preview_with_client sets what_if=true + reads OpenOrder.order_state; Tiger does NOT transmit under what_if (CONTEXT.md R1).
Done when: freeze empty + impl-shape correct + green gate reproduced ⇒ request explicit HUMAN CONFIRM, then squash-merge PR #23 (only pipeline-review merges); set card 01 status=done, current.json.stage=done; append journal seq=6 with the merge SHA. Do NOT merge without human confirm; do NOT force-push trunk.
On reject: card 01 status←todo, attempts++ (→1), journal seq=6; >=3 ⇒ pipeline-hunt.
<<< END

## seq=6 · 2026-07-05T14:37:36Z · review→done · completed · by=codex
done:   Human merge confirmation received. PR #23 squash-merged into main; card 01 marked done and
        current.json.stage set to done. Final trunk sanity cargo test will be run after metadata push.
output: PR #23 https://github.com/jackypanster/oh-my-ib/pull/23;
        squash-merge c5d9abb6a3fb807f092e8d75279670b8d6456f7d;
        .pipeline/order-preview/reviews/review-01.md
--- handoff ---
>>> NEXT
Feature order-preview is done. No next pipeline stage.
<<< END
