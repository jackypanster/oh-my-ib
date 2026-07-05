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
