# journal — write-path-semantics

## seq=1 · 2026-07-05T03:32:34Z · prd→arch · completed · by=claude (orchestrator)
done:   PRD for a one-time retroactive REFERENCE-BEHAVIOR AUDIT of the shipped write path
        (Reference-Port technique). Deliverable = one durable agent-first doc
        `docs/write-path-semantics.md` mapping every gateway-sent Order/Contract field
        (explicit + load-bearing ibapi Default) to reference semantics + verification tier
        (✅ paper-probe / 📖 doc-cite / ⚠️ UNVERIFIED), with a ⚠️ risk register carrying a
        runnable probe recipe per row. 4 operator decisions locked: D1 step-1-only (process
        change → SKILL-PROPOSAL, not this feature); D2 ship doc + ⚠️ register, probes deferred
        to a live US session; D3 home = docs/write-path-semantics.md; D4 depth = explicit +
        load-bearing defaults, inert tail one catch-all (~15-20 rows). Hard new evidence found:
        Order is built via `..Default::default()` → ibapi's CUSTOM `impl Default` (mod.rs:478)
        sets `transmit:true` (mod.rs:494); a derived Default would be false ⇒ orders silently
        never sent. Pure doc/audit — NO write-path code changes (a real wrong value found ⇒
        register here, fix as a separate feature, D6).
output: .pipeline/write-path-semantics/PRD.md, .pipeline/current.json
--- handoff ---
>>> NEXT
Run pipeline-arch on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
Model: frontier SOTA required (arch is a reasoning stage; grill-with-docs walks the design tree).
First: git pull --rebase; load repo config (.env if present, per CONTRACT step 2 — arch needs no gateway, so likely skip).
Read for context (before acting):
  - oh-my-ib/AGENTS.md + CLAUDE.md — repo conventions (agent-first authoring; write code lives ONLY in src/ib/trade.rs; read FIRST)
  - .pipeline/write-path-semantics/PRD.md — WHAT: the audit doc, coverage, 4 locked decisions D1-D6
  - src/ib/trade.rs — the write path: build_stk_order / build_option_order / build_combo_order / stamp_order_account / cancel + place_with_client choke point
  - .pipeline/order-account-stamp/docs/adr/0024-order-account-stamp.md §5 — the ad-hoc reference-port precedent this formalizes
  - .pipeline/order-account-stamp/journal.md seq=4/6 — the paper-probe pattern (Order.account already ✅)
  - ibapi-3.1.0/src/orders/mod.rs:73 (Order struct) + :478 (custom Default, transmit:true@494) — field-inventory source of truth
Your task (concrete, numbered):
  1. Grill the ARCHITECTURE of docs/write-path-semantics.md against the codebase: the exact row schema
     (7 columns per PRD success-criterion 3), section layout, and the ⚠️ risk-register format with
     per-row probe recipe (mirror ADR 0024 §5).
  2. DECIDE the top open question (PRD §Notes for arch): is the doc's correctness FROZEN by a coverage
     test — parse src/ib/trade.rs for every builder field set + assert each is a row in the doc — or
     left to review-by-reading (CONTRACT §Freeze coverage)? Record the decision + rationale in an ADR.
     This decision defines what pipeline-task can freeze.
  3. Enumerate the field inventory from ibapi mod.rs (struct + custom Default) + the Contract builders,
     and classify each: explicitly-set / load-bearing-default / inert-tail. This becomes the doc's row list.
  4. Emit arch.md + CONTEXT.md (domain glossary: verification tiers, "load-bearing default", probe recipe)
     + ADR(s). Do NOT author docs/write-path-semantics.md itself — that is the impl deliverable.
Feature gotchas (project-specific traps the next node MUST know):
  - DOC-ONLY feature: the "product" is a Markdown doc under docs/, not code in src/. The usual red-test
    surface is absent — arch MUST resolve freeze-ability (task 2) or the task stage has nothing to freeze.
  - Pure audit: NO change to src/ib/trade.rs order construction. An actual wrong field value found ⇒
    register it in the ⚠️ list, do NOT fix it here (D6 — a fix is its own feature/PR).
  - Probes are DEFERRED (D2): the doc ships with ⚠️ rows + recipes; running them needs a live :4002
    session (US hours). Do NOT design a merge gate that blocks on probe execution.
  - branch is `main` (not master). Only pipeline-review merges, human-confirmed.
  - SKILL-PROPOSAL (carry forward, do NOT act on it in this feature): pipeline-arch/pipeline-task —
    make a "reference-behavior" field-semantics subsection REQUIRED for any write-path card before its
    red test is frozen; goes via pipeline-improve against jackypanster/pipeline, not this repo's PR.
Done when: arch.md + CONTEXT.md + ADR(s) landed on trunk; freeze-ability decided; field inventory
classified. On success: current.json.stage=arch, then run pipeline-task.
On failure: attempts++; >=3 ⇒ blocked ⇒ run pipeline-hunt.
<<< END

## seq=2 · 2026-07-05T03:43:07Z · arch→task · completed · by=claude (orchestrator)
done:   Architecture for the doc-only reference-behavior audit. RESOLVED the top open question
        (freeze-ability): the doc is a TEST-GUARDED LIVING artifact, not review-by-reading only —
        ADR 0025. Frozen spec = tests/write_path_semantics_doc.rs (coverage + default-canary):
        (a) doc exists via runtime read_to_string — RED now; (b) every required field token has a
        row + tier marker; (c) anti-rot source-scan of src/ib/trade.rs asserts each set field is
        documented; (d) default-canary pins ibapi Order::default() load-bearing values
        (transmit==true, outside_rth==false, what_if==false, tif==Day, display_size==Some(0),
        origin==Customer, exempt_code==-1) — the transmit-catastrophe guard. Deliverable =
        docs/write-path-semantics.md (7-col table + ⚠️ risk register w/ probe recipes; account
        row ✅, transmit + combo-credit seeded ⚠️). Code-first verified: ibapi custom Default
        (mod.rs:478), Contract SMART/USD/mult-100 (builders.rs). One card. NO src/ change (D6).
output: .pipeline/write-path-semantics/arch.md, CONTEXT.md, docs/adr/0025-write-path-semantics-doc.md, current.json
--- handoff ---
>>> NEXT
Run pipeline-task on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
Model: frontier SOTA required (task is a reasoning stage; it authors the frozen red test).
First: git pull --rebase; load repo config (.env if present, per CONTRACT step 2 — task needs no gateway, skip).
Read for context (before acting):
  - oh-my-ib/AGENTS.md + CLAUDE.md — repo conventions (agent-first; write code ONLY in src/ib/trade.rs; read FIRST)
  - .pipeline/write-path-semantics/PRD.md — WHAT + decisions D1-D6
  - .pipeline/write-path-semantics/arch.md — the shape + "Freeze plan handed to task (advisory)" (USE IT VERBATIM)
  - .pipeline/write-path-semantics/docs/adr/0025-write-path-semantics-doc.md — the freeze design (§3 the test, §4 freeze coverage)
  - .pipeline/write-path-semantics/CONTEXT.md — glossary (verification tier, load-bearing default, default-canary, anti-rot scan)
  - src/ib/trade.rs — the write path the source-scan (c) targets
  - ibapi-3.1.0/src/orders/mod.rs:478 (custom Default) — the canary (d) values source of truth
Your task (concrete, numbered):
  1. ONE card, tasks/01.md. Decompose is trivial (single deliverable) — do NOT over-split.
  2. FREEZE COMMIT (spec-paths ONLY): write tests/write_path_semantics_doc.rs implementing ADR 0025 §3
     (a)+(b)+(c)+(d) using the arch.md required-field list + canary asserts VERBATIM. It MUST compile and
     FAIL now (doc absent ⇒ read_to_string Err ⇒ RED). GOTCHA: runtime std::fs::read_to_string(concat!(
     env!("CARGO_MANIFEST_DIR"), "/docs/write-path-semantics.md")), NEVER include_str! (absent-file =
     compile error, which would violate "spec must compile and FAIL"). This commit's hash = spec-rev.
  3. RECORD COMMIT (metadata only): tasks/01.md frontmatter — status: todo, attempts: 0,
     verify: `cargo test --test write_path_semantics_doc`, spec-paths: [tests/write_path_semantics_doc.rs],
     impl-paths: [docs/write-path-semantics.md], spec-rev: <freeze hash>. Assert spec-paths ∩ impl-paths = ∅.
     Advance current.json.stage=task. full-verify stays [cargo build, cargo test].
  4. Add a `## Freeze coverage` section to the card: FROZEN = doc exists + required-token coverage +
     tier markers + default-canary pin; REVIEW MUST READ = the SEMANTIC truth of each row's
     reference-semantics/boundary columns + that ⚠️ rows carry a runnable probe recipe (not machine-checkable).
Feature gotchas (project-specific traps the next node MUST know):
  - DOC-ONLY: impl writes docs/write-path-semantics.md (Markdown), NOT src/. The frozen test greens when
    the doc is complete. Do NOT freeze anything in src/ib/trade.rs — it does not change (D6).
  - read_to_string, NOT include_str! (see step 2) — the single most likely mistake.
  - Card verify is card-scoped (`--test write_path_semantics_doc`), never the full suite (CONTRACT multi-card rule),
    though this feature has one card.
  - Trunk suite goes RED from the freeze commit until impl merges — expected (one feature in flight, no CI).
  - branch is `main`. SKILL-PROPOSAL (carry forward, do NOT act): reference-behavior subsection required for
    write cards, via pipeline-improve — not this feature.
Done when: freeze commit (RED test) + record commit (card + stage) on trunk; spec-rev recorded on the card.
On success: current.json.stage=task, then run pipeline-impl. On failure: attempts++; >=3 ⇒ blocked ⇒ pipeline-hunt.
<<< END

## seq=3 · 2026-07-05T03:52:43Z · task→impl · completed · by=claude (orchestrator)
done:   Froze the spec in TWO commits (CONTRACT §Test ownership). FREEZE COMMIT
        1549375611870e690e1a6717df63de515b5691d5 = spec-rev: tests/write_path_semantics_doc.rs,
        verified COMPILES + FAILS (3 red: doc absent; 1 green: the default-canary, by design).
        RECORD COMMIT (this one): tasks/01.md (status=todo, spec-paths=[the test],
        impl-paths=[docs/write-path-semantics.md], disjoint ✓) + stage=task. One card.
        Task-author refinement of arch's advisory (c): the anti-rot scan is a reflection-free
        serde-diff of builder OUTPUT vs Order::default() (Order serializes plain field names, no
        rename) — robust where a trade.rs source regex is fragile; catches the same class (a new
        builder-set field nobody documented). full-verify unchanged [cargo build, cargo test].
output: .pipeline/write-path-semantics/tasks/01.md, current.json; spec-rev 1549375 (tests/write_path_semantics_doc.rs)
--- handoff ---
>>> NEXT
Run pipeline-impl on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
Model: capable-local OK (impl only) — this card writes ONE Markdown doc, no Rust logic.
First: git pull --rebase; load repo config (.env if present, per CONTRACT step 2 — impl needs no gateway, skip).
Read for context (before acting):
  - oh-my-ib/AGENTS.md + CLAUDE.md — repo conventions (agent-first authoring; read FIRST)
  - .pipeline/write-path-semantics/tasks/01.md — THE CARD: what to write, the row/tier/probe guidance, Freeze coverage
  - .pipeline/write-path-semantics/docs/adr/0025-write-path-semantics-doc.md — the doc schema (§1) + freeze design
  - .pipeline/write-path-semantics/PRD.md + CONTEXT.md — decisions D1-D6 + glossary
  - tests/write_path_semantics_doc.rs — the FROZEN spec you must green (do NOT edit it)
  - src/ib/trade.rs — READ ONLY: the builders/fields the doc audits (do NOT modify — D6)
  - ibapi-3.1.0/src/orders/mod.rs:478 (Default) + contracts/builders.rs — reference values to cite (📖)
Your task (concrete, numbered):
  1. Cut feat/write-path-semantics from trunk (carries spec-rev 1549375).
  2. Write docs/write-path-semantics.md per tasks/01.md: 7-col table (~15-20 rows) + `## ⚠️ Risk register`
     with a runnable probe recipe per ⚠️ row. account=✅ (cite order-account-stamp probe); combo net-credit
     + display_size=Some(0) = ⚠️ with recipes; the rest 📖 with ibapi/IB citations.
  3. Green the card: `cargo build` && `cargo test --test write_path_semantics_doc` all pass. The serde-diff
     test (c) passes once every builder-set field has a row; (b) once every required token is present.
  4. Edit ONLY docs/write-path-semantics.md (impl-paths). Do NOT touch tests/write_path_semantics_doc.rs
     (spec freeze gate) or any src/ file (D6). Open the PR.
Feature gotchas:
  - impl-paths = docs/write-path-semantics.md ONLY. Touching the frozen test or src/ = a freeze-gate reject.
  - A field value you think is WRONG ⇒ register a ⚠️ row, do NOT fix code (D6 — separate feature).
  - The ⚠️ probes are DEFERRED (D2): write the recipes, do NOT run them (need live :4002 US session).
  - Doc must satisfy the serde-diff (c): if unsure which fields the builders set, run the test and read the
    failure — it NAMES the undocumented field.
  - branch main; only pipeline-review merges (human-confirmed).
Done when: cargo test --test write_path_semantics_doc GREEN on feat/write-path-semantics; PR open.
On success: current.json.stage=impl, then run pipeline-review. On failure: attempts++; >=3 ⇒ blocked ⇒ pipeline-hunt.
<<< END

## seq=4 · 2026-07-05T11:55:00Z · impl · completed · by=claude (pipeline-impl)
done:   Card 01 GREEN. Cut feat/write-path-semantics from main (carries spec-rev
        1549375). Wrote docs/write-path-semantics.md (impl-paths, DOC-ONLY — no
        src/ change, D6): 7-col field inventory (~20 rows: explicitly-set Order
        fields action/total_quantity/order_type/tif/limit_price/account; load-bearing
        ibapi Default fields transmit/outside_rth/display_size/what_if/origin/
        exempt_code; Contract surface symbol/security_type/exchange/currency/
        multiplier/strike/right; combo net-credit sign; one inert-tail catch-all),
        a placement choke-point note (stamp_order_account @ place_with_client,
        trade.rs:317), and a `## ⚠️ Risk register` with a runnable `omi --paper`
        probe recipe per ⚠️ row (display_size=Some(0) — ibapi carries the
        `// TODO - default to None?` at mod.rs:498; combo net-credit sign — ADR
        0021's negative=credit assumption). Tier assignment: account=✅ (paper
        probe via order-account-stamp seq=4/6); display_size + combo credit = ⚠️;
        all other rows 📖 with ibapi source-line citations (transmit:true@494,
        outside_rth:false@500, what_if:false@562, tif:Day@490, origin:Customer@516,
        exempt_code:-1@519, SMART/USD/mult-100 @ builders.rs:23-24/93-95/513).
        VERIFY card-scoped: cargo build ✓; cargo test --test write_path_semantics_doc
        ⇒ 4/4 pass ((a) structure, (b) required tokens, (c) anti-rot serde-diff,
        (d) default-canary). Probes DEFERRED (D2) — recipes only, not executed.
        PR #22 opened (gh authenticated, repo scope).
output: docs/write-path-semantics.md (on feat/write-path-semantics); PR #22; .pipeline/write-path-semantics/tasks/01.md (status=review), .pipeline/current.json (stage=impl)
--- handoff ---
>>> NEXT
Feature write-path-semantics has ONE card (this one); it is now status=review.
        NO todo/in-progress cards remain ⇒ run pipeline-review ONCE on the complete feature.
Run pipeline-review on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git base=main branch=feat/write-path-semantics pr=#22
Model: frontier SOTA recommended (review is a reasoning stage; semantic truth of each row is read-by-eye).
First: git pull --rebase; load repo config (.env if present, per CONTRACT step 2 — review needs no gateway, skip).
Read for context (before acting):
  - oh-my-ib/AGENTS.md + CLAUDE.md — repo conventions (agent-first; write code ONLY in src/ib/trade.rs)
  - .pipeline/write-path-semantics/PRD.md + arch.md + CONTEXT.md — WHAT + decisions D1-D6 + glossary
  - .pipeline/write-path-semantics/docs/adr/0025-write-path-semantics-doc.md — §4 "Freeze coverage" = the review checklist
  - .pipeline/write-path-semantics/tasks/01.md — the card (impl-paths, Freeze coverage section)
  - tests/write_path_semantics_doc.rs — the FROZEN spec (4 guards; do NOT edit)
  - docs/write-path-semantics.md — the deliverable under review (the diff is this one file)
  - src/ib/trade.rs — READ ONLY cross-check for the doc's field citations
Your task (concrete, numbered):
  1. Diff feat/write-path-semantics against main: expect EXACTLY one new file (docs/write-path-semantics.md).
     Any src/ or tests/ change ⇒ freeze-gate reject (impl-paths = the doc ONLY, D6).
  2. SEMANTIC review (ADR 0025 §4 — NOT machine-checkable): for EACH row, the reference-semantics and
     boundary columns are TRUE against ibapi source + IB TWS API reference; tier assignments are HONEST
     (every 📖 has a real citation, ✅ has a real probe, ⚠️ has a real gap + recipe).
  3. Every ⚠️ row (display_size=Some(0), combo net-credit sign) carries a RUNNABLE probe recipe on :4002
     (exact omi command + the observable that confirms/refutes + the fallback). Probes are deferred (D2) —
     review confirms the recipe is runnable, NOT that it was run.
  4. Run full-verify (cargo build && cargo test) on the branch — the trunk suite was RED from the freeze
     commit; this feature turning it GREEN is the final freeze gate (CONTRACT §State authority).
  5. Human-confirm the merge. Only pipeline-review merges.
Feature gotchas:
  - DOC-ONLY feature: the diff is a Markdown doc, not Rust. Do not be surprised that cargo test passes
    purely by the doc existing and naming the right tokens.
  - account is stamped post-build (place_with_client choke point, trade.rs:317) — NOT builder output, so
    invisible to the anti-rot serde-diff (c); covered by required-token (b) + this read-by-eye review.
  - The ⚠️ probes are the deferred live acceptance (D2): they populate the risk register; running them is
    an operator lifecycle in a US session, not a merge gate.
Done when: full-verify GREEN on the branch; semantic review passes; human confirms ⇒ merge PR #22.
On success: trunk GREEN (freeze red cleared); feature complete. On failure: review rejects with reasons;
re-route to pipeline-impl (attempts<3) or pipeline-hunt (>=3 blocked).
<<< END

## seq=5 · 2026-07-05T05:12:13Z · review→impl · failed · by=codex-reviewer
done:   review-01 CHANGES REQUESTED for PR #22 / feat/write-path-semantics tip 5364482.
        Freeze gate PASS (spec-rev 1549375..tip over tests/write_path_semantics_doc.rs empty);
        net PR diff only adds docs/write-path-semantics.md; detached PR-head full-verify PASS
        (`cargo build`, `cargo test`). Blocking findings: (1) both ⚠️ probe recipes use
        `omi --paper`, but the CLI has no --paper flag, so the recipes fail at argument parsing;
        (2) combo net-limit row/probe flatten IBKR's action-relative combination pricing into
        "negative = credit" and use `--action sell --limit -0.05`, conflicting with IBKR's
        documented sell-credit positive-price convention. Card 01 review→todo, attempts 0→1.
output: .pipeline/write-path-semantics/reviews/review-01.md
--- handoff ---
>>> NEXT
Run pipeline-impl on a FRESH session for the write-path-semantics review-01 rejection.
repo=git@github.com:jackypanster/oh-my-ib.git base=main branch=feat/write-path-semantics pr=#22
Model: capable-local OK for the focused Markdown retry; use frontier SOTA if changing combo semantics.
First: git pull --rebase; load repo config (.env if present, per CONTRACT step 2 — impl needs no gateway).
Read for context:
  - oh-my-ib/AGENTS.md + CLAUDE.md — repo conventions (agent-first; no secrets; review owns metadata)
  - .pipeline/write-path-semantics/reviews/review-01.md — blocking findings and passed evidence
  - .pipeline/write-path-semantics/tasks/01.md — card 01 now todo, attempts=1; impl-paths still docs/write-path-semantics.md only
  - docs/write-path-semantics.md on feat/write-path-semantics — the doc to fix
  - tests/write_path_semantics_doc.rs — frozen spec; do NOT edit
  - src/cli.rs — CLI surface: paper is default, --live selects live; there is no --paper
  - src/ib/trade.rs — write-path source, read-only cross-check
Your task:
  1. On feat/write-path-semantics, edit ONLY docs/write-path-semantics.md.
  2. Replace invalid `omi --paper ...` probe commands with valid paper commands: plain `omi ...`
     (default :4002) or explicit `omi --port 4002 ...`; ensure every ⚠️ recipe is copy-paste runnable.
  3. Correct combo net-limit semantics: IBKR/TWS combination-order pricing is action-relative.
     Buying a credit spread uses a negative limit; selling a spread and receiving cash uses a
     positive limit. Keep Tiger-specific acceptance as ⚠️ if unprobed, but do not state the
     global "negative = credit" rule as reference semantics.
  4. Adjust the combo probe accordingly: either test a BUY credit spread with a negative limit, or
     test a SELL credit spread with a positive limit. If testing the current `--action sell --limit -0.05`
     behavior, label it as a suspected divergence from IBKR docs.
  5. Re-run card verify (`cargo build`, `cargo test --test write_path_semantics_doc`) and full verify
     (`cargo build`, `cargo test`), then set card 01 back to review, append the next journal entry,
     push branch + trunk metadata, and hand off to pipeline-review.
Feature gotchas:
  - DOC-ONLY feature: no `src/` changes and no frozen test edits. A code fix is a separate feature (D6).
  - The frozen doc test will pass even if the probe recipe is not runnable; review-01 is a semantic
    reviewed-by-reading rejection.
  - `omi --help` is the source of truth for CLI flags; paper is the default gateway port 4002.
Done when: docs/write-path-semantics.md is fixed on PR #22, card verify + full suite are green,
card 01 is back to review with attempts still 1, journal seq=6 is pushed, and pipeline-review can rerun.
On failure: attempts++; attempts>=3 ⇒ blocked ⇒ pipeline-hunt.
<<< END

## seq=6 · 2026-07-05T13:25:00Z · impl · completed · by=claude (pipeline-impl)
done:   Review-01 fixes applied. Edited ONLY docs/write-path-semantics.md on
        feat/write-path-semantics (commit 013e84d, PR #22 updated). Three concrete
        fixes (review found 2, expiry-format was a 3rd discovered while fixing):
        (1) Dropped the non-existent `--paper` flag from every probe recipe. Paper
            is the DEFAULT gateway port :4002 (cli.rs:28-33, config.rs:12); live is
            opt-in via --live. Recipes now use plain `omi ...`.
        (2) Fixed the expiry format in both recipes: `20260918` (YYYYMMDD), not the
            dashed `2026-09-18` form, which is a frozen usage error
            (option-orders::dashed_expiry_is_usage_error).
        (3) Rewrote the combo `credit` row: IBKR TWS "Notes on Combination Orders"
            defines the net-limit sign ACTION-RELATIVE — selling a credit spread
            takes a POSITIVE limit, only BUYing a credit spread takes a negative
            one. The prior "negative = credit" was a flattening. Flagged omi's
            CLI help string `--limit ... negative = credit` (cli.rs:236) as a
            SIMPLIFICATION that misleads for SELL-action combos. Kept Tiger's
            enforcement + the help-string divergence as ⚠️. The probe now tests
            BOTH coherent pairings (positive-limit SELL per IBKR vs negative-limit
            SELL per omi's help string) so the operator can distinguish the two
            hypotheses. Did NOT change cli.rs or trade.rs (D6 — separate feature).
        VERIFY card-scoped: cargo build ✓; cargo test --test write_path_semantics_doc
        ⇒ 4/4 pass. attempts stays 1 (rejection didn't reset it; this is the same
        card's successful retry within budget). Probes still DEFERRED (D2).
output: docs/write-path-semantics.md (commit 013e84d on feat/write-path-semantics, PR #22); .pipeline/write-path-semantics/tasks/01.md (status=review), .pipeline/current.json (stage=impl)
--- handoff ---
>>> NEXT
Feature write-path-semantics has ONE card; it is back to status=review after the
        review-01 fixes. NO todo/in-progress cards remain ⇒ run pipeline-review ONCE on the complete feature.
Run pipeline-review on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git base=main branch=feat/write-path-semantics pr=#22
Model: frontier SOTA recommended (review is a reasoning stage; the combo sign semantics are subtle).
First: git pull --rebase; load repo config (.env if present, per CONTRACT step 2 — review needs no gateway, skip).
Read for context (before acting):
  - oh-my-ib/AGENTS.md + CLAUDE.md — repo conventions (agent-first; write code ONLY in src/ib/trade.rs)
  - .pipeline/write-path-semantics/reviews/review-01.md — the prior rejection (2 findings)
  - .pipeline/write-path-semantics/journal.md seq=5 (review rejection) + seq=6 (this fix) — what changed and why
  - .pipeline/write-path-semantics/docs/adr/0025-write-path-semantics-doc.md — §4 "Freeze coverage" = the review checklist
  - .pipeline/write-path-semantics/tasks/01.md — the card (impl-paths, Freeze coverage, Review 01 rejection — required fixes)
  - tests/write_path_semantics_doc.rs — the FROZEN spec (4 guards; do NOT edit)
  - docs/write-path-semantics.md — the deliverable under review (the diff is this one file vs main)
  - src/cli.rs + src/ib/trade.rs — READ ONLY cross-check for the doc's CLI/sign citations
Your task (concrete, numbered):
  1. Diff feat/write-path-semantics against main: expect EXACTLY one new file (docs/write-path-semantics.md).
     Any src/ or tests/ change ⇒ freeze-gate reject (impl-paths = the doc ONLY, D6).
  2. Confirm review-01 findings are RESOLVED: (a) no `omi --paper` anywhere — recipes use plain `omi ...`
     on the default :4002; (b) the combo `credit` row states IBKR's ACTION-RELATIVE convention (not a
     global "negative = credit"), and the probe tests coherent action/sign pairings.
  3. SEMANTIC review (ADR 0025 §4): for EACH row, the reference-semantics and boundary columns are TRUE;
     tier assignments are HONEST (every 📖 has a real citation, ✅ a real probe, ⚠️ a real gap + recipe).
  4. Every ⚠️ row (display_size=Some(0), combo net-credit sign) carries a RUNNABLE probe recipe on :4002
     with YYYYMMDD expiry and the correct paper invocation shape. Probes are deferred (D2) — review
     confirms the recipe is runnable, NOT that it was run.
  5. Run full-verify (cargo build && cargo test) on the branch — the final freeze gate.
  6. Human-confirm the merge. Only pipeline-review merges.
Feature gotchas:
  - The combo sign semantics are the subtle point: IBKR is action-relative (SELL-credit = positive
    limit); omi's CLI help string `negative = credit` (cli.rs:236) is a simplification that only holds
    for BUY-action combos. The doc now flags this; a code/help-string fix is a separate feature (D6).
  - The frozen doc test passes structurally (doc exists + tokens + serde-diff + canary); the SEMANTIC
    truth of each row is review-by-reading — that is what rejected seq=4 and what must pass now.
  - account is stamped post-build (place_with_client, trade.rs:317) — covered by required-token (b) + review.
Done when: full-verify GREEN on the branch; semantic review passes (review-01 resolved); human confirms ⇒ merge PR #22.
On success: trunk GREEN; feature complete. On failure: review rejects with reasons; attempts++ (currently 1);
attempts>=3 ⇒ blocked ⇒ pipeline-hunt.
<<< END

## seq=7 · 2026-07-05T13:33:51Z · review→impl · failed · by=codex-reviewer
done:   review-02 CHANGES REQUESTED for PR #22 / feat/write-path-semantics tip 013e84d.
        Freeze gate PASS (spec-rev 1549375..tip over tests/write_path_semantics_doc.rs empty);
        net PR diff only adds docs/write-path-semantics.md; detached PR-head full-verify PASS
        (`cargo build`, `cargo test`). review-01 command-shape findings are resolved:
        no `--paper`, YYYYMMDD expiries, and all probe commands parse through to dead-port
        connection envelopes. Blocking finding: combo probe still has wrong live semantics.
        IBKR documents whole-combo action as multiplying/inverting leg actions, but the doc's
        "SELL-credit per IBKR" recipe uses `--action sell` with `SELL lower / BUY higher`, so it
        probes the inverse effective spread rather than the stated credit spread. Card 01
        review→todo, attempts 1→2.
output: .pipeline/write-path-semantics/reviews/review-02.md
--- handoff ---
>>> NEXT
Run pipeline-impl on a FRESH session for the write-path-semantics review-02 rejection.
repo=git@github.com:jackypanster/oh-my-ib.git base=main branch=feat/write-path-semantics pr=#22
Model: capable-local OK for the focused Markdown retry; frontier SOTA recommended if unsure about combo semantics.
First: git pull --rebase; load repo config (.env if present, per CONTRACT step 2 — impl needs no gateway).
Read for context:
  - oh-my-ib/AGENTS.md + CLAUDE.md — repo conventions (agent-first; no secrets)
  - .pipeline/write-path-semantics/reviews/review-02.md — blocking finding and passed evidence
  - .pipeline/write-path-semantics/reviews/review-01.md — prior findings, now mostly resolved
  - .pipeline/write-path-semantics/tasks/01.md — card 01 now todo, attempts=2; impl-paths still docs/write-path-semantics.md only
  - docs/write-path-semantics.md on feat/write-path-semantics — the doc to fix
  - tests/write_path_semantics_doc.rs and tests/option_combo_command.rs — frozen specs; do NOT edit
  - src/ib/trade.rs — read-only: build_combo_order stores whole-order action plus per-leg actions
Your task:
  1. On feat/write-path-semantics, edit ONLY docs/write-path-semantics.md.
  2. Keep review-01 fixes intact: no `omi --paper`, use YYYYMMDD expiries, paper is default :4002.
  3. Fix the combo credit probe semantics. Document that `Order.action` is the whole-combo side and
     IBKR explains combo action as multiplying/inverting leg actions. Then use coherent probes:
     - BUY credit spread: `--action buy` + `SELL lower / BUY higher` + negative limit.
     - SELL spread for credit: use the opposite leg vector (`BUY lower / SELL higher`) with
       `--action sell` + positive limit.
     The recipe must name the effective position it creates; do not label `--action sell` +
     `SELL lower / BUY higher` as SELL-credit.
  4. Re-run card verify (`cargo build`, `cargo test --test write_path_semantics_doc`) and full verify
     (`cargo build`, `cargo test`), then set card 01 back to review, append journal seq=8, push branch
     + trunk metadata, and hand off to pipeline-review.
Feature gotchas:
  - attempts is now 2. A third review rejection blocks this card and routes to pipeline-hunt, so keep the
    retry narrow and grounded in IBKR docs + local `build_combo_order` behavior.
  - DOC-ONLY feature: no `src/` changes and no frozen test edits. Code/help-string fixes are separate features (D6).
  - The frozen doc test is structural; the combo risk-register semantics are review-by-reading.
Done when: docs/write-path-semantics.md has coherent combo probe semantics, full verify is green,
card 01 is back to review with attempts still 2, journal seq=8 is pushed, and pipeline-review can rerun.
On failure: attempts++; attempts>=3 ⇒ blocked ⇒ pipeline-hunt.
<<< END

## seq=8 · 2026-07-05T13:50:00Z · impl · completed · by=claude (pipeline-impl)
done:   Review-02 fix applied. Edited ONLY docs/write-path-semantics.md on
        feat/write-path-semantics (commit 846e719, PR #22 updated). Three concrete
        fixes (one finding, two supporting clarifications):
        (1) action row: added an explicit combo scalar-vector note to the boundary
            column — Order.action multiplies each ComboLeg.action (--action buy keeps
            leg actions as-written; --action sell INVERTS them). This is the IBKR
            TWS combo-lesson semantics the prior version was missing.
        (2) credit row: rewrote the IBKR reference-semantics cell to spell out the
            scalar-vector interaction concretely for a call credit spread (L<H):
            BUY-credit = --action buy + SELL L / BUY H + negative limit (effective
            legs unchanged); SELL-credit = --action sell + BUY L / SELL H + positive
            limit (effective legs inverted to SELL L / BUY H). Flagged omi's
            cli.rs:236 'negative = credit' help string as a simplification that
            ignores BOTH the action-relative sign AND the scalar-vector leg inversion.
        (3) Risk register: rewrote the combo entry as THREE structurally coherent
            probes on the same call credit spread (L=240, H=250), each NAMING the
            effective position it actually creates under the IBKR scalar-vector model:
              - probe 1 BUY-credit per IBKR (--action buy + SELL 240 / BUY 250 + -0.05)
                → effective legs unchanged = bear call = credit spread, bought. Matches
                the frozen test negative_net_limit_is_a_credit_and_builds build shape.
              - probe 2 SELL-credit per IBKR (--action sell + BUY 240 / SELL 250 + 0.05)
                → effective legs re-inverted to SELL 240 / BUY 250 = same credit spread, sold.
              - probe 3 divergence probe (--action sell + BUY 240 / SELL 250 + -0.05, omi
                help-string literal) → credit-spread legs sold under a SELL-debit sign per
                IBKR; if Tiger rejects/reprices this where probe 2 was accepted, omi's
                help-string simplification is a confirmed divergence.
            Added a 'reading the triplet' paragraph mapping outcome combinations to which
            convention Tiger follows. The mislabelled `--action sell + SELL 240 / BUY 250`
            recipe (effective bull-call DEBIT sold) is gone.
        VERIFY card-scoped: cargo build ✓; cargo test --test write_path_semantics_doc
        ⇒ 4/4 pass. attempts stays 2. Probes still DEFERRED (D2). Code unchanged (D6):
        build_combo_order still stores Order.action and ComboLeg.action independently
        (trade.rs:563,574); the scalar-vector interpretation is a gateway-semantics
        question the doc now states and the probes isolate.
output: docs/write-path-semantics.md (commit 846e719 on feat/write-path-semantics, PR #22); .pipeline/write-path-semantics/tasks/01.md (status=review), .pipeline/current.json (stage=impl)
--- handoff ---
>>> NEXT
Feature write-path-semantics has ONE card; it is back to status=review after the
        review-02 fixes. NO todo/in-progress cards remain ⇒ run pipeline-review ONCE on the complete feature.
        attempts is 2 — a third semantic rejection blocks this card and routes to pipeline-hunt.
Run pipeline-review on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git base=main branch=feat/write-path-semantics pr=#22
Model: frontier SOTA recommended (review is a reasoning stage; the combo scalar-vector semantics are subtle).
First: git pull --rebase; load repo config (.env if present, per CONTRACT step 2 — review needs no gateway, skip).
Read for context (before acting):
  - oh-my-ib/AGENTS.md + CLAUDE.md — repo conventions (agent-first; write code ONLY in src/ib/trade.rs)
  - .pipeline/write-path-semantics/reviews/review-01.md + review-02.md — the two prior rejections
  - .pipeline/write-path-semantics/journal.md seq=7 (review-02 rejection) + seq=8 (this fix) — what changed and why
  - .pipeline/write-path-semantics/docs/adr/0025-write-path-semantics-doc.md — §4 "Freeze coverage" = the review checklist
  - .pipeline/write-path-semantics/tasks/01.md — the card (impl-paths, Freeze coverage)
  - tests/write_path_semantics_doc.rs + tests/option_combo_command.rs — frozen specs; do NOT edit
  - docs/write-path-semantics.md — the deliverable under review
  - src/cli.rs (cli.rs:236 help string) + src/ib/trade.rs (build_combo_order:563,574) — READ ONLY cross-check
Your task (concrete, numbered):
  1. Diff feat/write-path-semantics against main: expect EXACTLY one new file (docs/write-path-semantics.md).
     Any src/ or tests/ change ⇒ freeze-gate reject (impl-paths = the doc ONLY, D6).
  2. Confirm review-02 finding is RESOLVED: the combo probe pair (now a triplet) uses leg vectors
     COHERENT with the whole-order action under the IBKR scalar-vector model. Each probe NAMES the
     effective position it creates:
     - probe 1 BUY-credit: --action buy + SELL L / BUY H + negative limit → effective unchanged.
     - probe 2 SELL-credit: --action sell + BUY L / SELL H + positive limit → effective re-inverted.
     - probe 3 divergence: --action sell + BUY L / SELL H + negative limit → SELL-debit sign on credit legs.
     No recipe pairs --action sell with SELL lower / BUY higher legs labeled as SELL-credit.
  3. SEMANTIC review (ADR 0025 §4): each row's reference-semantics and boundary columns are TRUE;
     tier assignments HONEST; every ⚠️ row carries a runnable probe with YYYYMMDD expiry and the
     correct paper invocation shape (plain `omi ...`, default :4002).
  4. Run full-verify (cargo build && cargo test) on the branch — the final freeze gate.
  5. Human-confirm the merge. Only pipeline-review merges.
Feature gotchas:
  - The combo scalar-vector model is the subtle point: Order.action is a scalar that IBKR multiplies
    into each ComboLeg.action; --action sell inverts the leg vector. omi's code stores both fields
    independently (trade.rs:563,574) and passes them through — the doc now states this, the probes
    isolate Tiger's interpretation. A code/help-string fix is a separate feature (D6).
  - The frozen doc test passes structurally; the SEMANTIC truth of the combo section is what rejected
    seq=6 and what must pass now.
  - account is stamped post-build (place_with_client, trade.rs:317) — covered by required-token (b) + review.
Done when: full-verify GREEN on the branch; semantic review passes (review-02 resolved); human confirms ⇒ merge PR #22.
On success: trunk GREEN; feature complete. On failure: review rejects with reasons; attempts++ (currently 2);
attempts>=3 ⇒ blocked ⇒ pipeline-hunt.
<<< END

## seq=9 · 2026-07-05T13:55:55Z · impl→review · completed · by=codex-reviewer
done:   review-03 APPROVED for PR #22 / feat/write-path-semantics tip 846e719.
        Freeze gate PASS (spec-rev 1549375..tip over tests/write_path_semantics_doc.rs empty);
        net PR diff only adds docs/write-path-semantics.md; detached PR-head full-verify PASS
        (`cargo build`, `cargo test`). review-01 and review-02 findings are resolved: no `--paper`,
        YYYYMMDD expiries, command recipes parse through to dead-port connection envelopes, and the
        combo risk-register triplet now uses coherent action/leg/sign semantics under IBKR's
        scalar-vector model. Awaiting explicit human confirmation before merge.
output: .pipeline/write-path-semantics/reviews/review-03.md
--- handoff ---
>>> NEXT
Human confirmation required before merge.
repo=git@github.com:jackypanster/oh-my-ib.git base=main branch=feat/write-path-semantics pr=#22
Review verdict: APPROVED in .pipeline/write-path-semantics/reviews/review-03.md.
Merge preconditions already satisfied:
  - freeze gate GREEN: spec-rev 1549375611870e690e1a6717df63de515b5691d5 → PR head 846e719 over tests/write_path_semantics_doc.rs is empty
  - feature completeness guard GREEN: only card 01 exists and status=review
  - full-suite gate GREEN on detached PR-head worktree: cargo build; cargo test
  - semantic review GREEN: review-01 and review-02 findings resolved
If the operator says exactly to proceed/merge/go, run pipeline-review merge completion:
  1. Re-read PR #22 head; if head changed, rerun freeze gate + full-verify before merging.
  2. Squash-merge PR #22 via the GitHub forge adapter and delete the merged feat/write-path-semantics branch.
  3. On main, set every card in write-path-semantics to status=done and set .pipeline/current.json stage=done.
  4. Append journal seq=10 review→done completed with the merge SHA/PR URL.
  5. Commit + push the trunk metadata completion commit.
Hard rules:
  - Do NOT merge without explicit human confirmation.
  - Do NOT force-push trunk/shared refs.
  - Do NOT skip review-NN, card→done, current.json.stage=done, or the final journal entry after merge.
<<< END
