# Run journal — option-close

## seq=1 · 2026-07-04T07:37:49Z · prd→arch · completed · by=claude-fable-5
done:   PRD decision-complete: close-by-conid verb (side/qty derived from held position,
        LMT/DAY, ADR 0017/0018 verbatim) + positions/brief 14-key row identity enrichment
        (sec_type/expiry/strike/right/multiplier, nulls on non-OPT). D4 rebuild+conid-assert
        placement path (portfolio-contract resubmit REJECTED as unverified on Tiger).
        Operator /think 2026-07-04: scope + full-auto authorized.
output: .pipeline/option-close/PRD.md
--- handoff ---
>>> NEXT
Run pipeline-arch on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
Model: frontier SOTA required — operator assigns the bot; the pipeline can't verify the model.
First: git pull --rebase; no repo .env (real config outside repo, not needed for arch).
Read for context (before acting):
  - AGENTS.md — repo conventions (agent-first authoring, hard safety rules)
  - .pipeline/option-close/PRD.md — what + locked decisions D1-D8
  - src/ib/positions.rs — position_row shared seam (brief.rs consumes it — parity ripple)
  - src/ib/trade.rs — place_with_client core, option builder path, validation-ordering idiom
  - src/ib/option_quote.rs — pub(crate) parse_expiry/normalize_right (reuse, ADR 0020 D6)
  - docs/adr/0017..0021 — write-path safety machinery (verbatim reuse, zero new machinery)
Your task (concrete, numbered):
  1. grill-with-docs the design tree: (a) position_row 9→14 key mapping (SecurityType→string,
     null semantics, brief parity), (b) option_close single-connect data flow
     (drain→match→rebuild→contract_details conid assert→place_with_client), (c) exact
     OptionCloseArgs/CLI + 10-key ack struct, (d) AGENTS.md+CLAUDE.md two-text amendment.
  2. Compute the CLAUDE.md <900B budget BEFORE freezing amendment text (option-orders lesson).
  3. Emit arch.md + CONTEXT.md update + ADR 0022; commit once with journal seq=2; push.
Feature gotchas (project-specific traps the next node MUST know):
  - Single-connect invariant: ONE client for drain+resolve+place (option-combo review lesson —
    a second same-client-id connect wedges the gateway).
  - positions row shape is SHARED with brief — two frozen suites re-freeze (task owns).
  - SecurityType/OptionRight are non_exhaustive ibapi enums; multiplier is String; expiry row
    output is raw passthrough string but the rebuild needs parse_expiry's (y,m,d).
  - expiry/strike/right come from ibapi decode_contract on the portfolio stream — Tiger's actual
    field content is UNVERIFIED (PRD risk 1); D4 conid assert is the guard, keep it before place.
Done when: arch.md + CONTEXT.md + ADR 0022 on trunk (journal seq=2 riding the same commit).
On success: run pipeline-task. On failure: attempts++; >=3 ⇒ blocked ⇒ run pipeline-hunt.
<<< END

## seq=2 · 2026-07-04T07:43:39Z · arch→task · completed · by=claude-fable-5
done:   arch.md + CONTEXT.md + ADR 0022 landed. Grill findings: row shape NEVER frozen before
        (position_row pub(crate) — card 01 is the FIRST freeze, NO existing-suite re-freeze;
        refines PRD criterion 10); rebuild reuses build_option_order VERBATIM (zero new builder
        code); SecurityType Display = wire code ("STK"/"OPT"; contract.rs Debug variant = legacy,
        untouched); CLAUDE.md amendment computed 861+15=876B < 900 ✓.
output: .pipeline/option-close/arch.md, CONTEXT.md, docs/adr/0022-option-close-by-conid.md
--- handoff ---
>>> NEXT
Run pipeline-task on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
Model: frontier SOTA required — operator assigns the bot; the pipeline can't verify the model.
First: git pull --rebase; no repo .env.
Read for context (before acting):
  - AGENTS.md — repo conventions
  - .pipeline/option-close/PRD.md — what + D1-D8
  - .pipeline/option-close/arch.md — §Freeze plan handed to task (advisory), exact seams/keys
  - .pipeline/option-close/docs/adr/0022 — binding decisions
  - tests/option_orders_command.rs + tests/option_combo_command.rs — freeze style precedent
Your task (concrete, numbered):
  1. Two cards: 01 positions identity enrichment (tests/positions_row.rs), 02 option-close verb
     (tests/option_close_command.rs). spec-paths = the two test files; impl-paths per arch §files.
  2. ONE freeze commit for the WHOLE feature (both cards' red tests, compiling, RED on missing
     symbols only — verbatim-compile pre-verify via stub module, delete stub pre-commit), then
     ONE record commit (cards + current.json stage=task + full-verify [cargo build, cargo test]).
  3. Card verify card-scoped: cargo test --test positions_row / --test option_close_command.
  4. Journal seq=3 rides the record commit; push both commits.
Feature gotchas (project-specific traps the next node MUST know):
  - Deliberate gate-pass omission in card 02 spec (env+live+dead-port test would REALLY order
    if a live gateway is up — option-orders precedent; shared gate covered by stk twin tests).
  - position_row promotion pub(crate)→pub is part of card 01 IMPL, so the spec test imports
    oh_my_ib::ib::positions::position_row and is RED on visibility/missing keys until impl.
  - derive_close/shape_option_close_ack live in trade.rs (write module) — spec imports must
    match; ack echoes RESOLVED row identity, action is DERIVED side.
  - Synthetic AccountPortfolioValue/Contract construction: all ibapi fields pub; SecurityType/
    OptionRight non_exhaustive — use ..Default::default() spread, never exhaustive match.
Done when: freeze commit (= spec-rev) + record commit on trunk, both pushed, journal seq=3.
On success: run pipeline-impl (π on the omp pane, capable-local OK). On failure: attempts++;
>=3 ⇒ blocked ⇒ run pipeline-hunt.
<<< END

## seq=3 · 2026-07-04T07:49:02Z · task→impl · completed · by=claude-fable-5
done:   spec FROZEN: freeze commit 286eb6a (= spec-rev) carries BOTH cards' red tests
        (tests/positions_row.rs 6 tests + tests/option_close_command.rs 21 tests; RED on
        unresolved oh_my_ib imports only; verbatim-compile pre-verified via stub scratch,
        deleted pre-commit). TWO cards at todo/0. Same deliberate gate-pass omission as
        option-orders/combo (live-order hazard). full-verify recorded.
output: .pipeline/option-close/tasks/01.md, tasks/02.md, tests/positions_row.rs,
        tests/option_close_command.rs (spec-rev 286eb6a)
--- handoff ---
>>> NEXT
Run pipeline-impl on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
Model: capable-local OK (impl) — π on the omp pane; operator/orchestrator assigns.
First: git pull --rebase; no repo .env.
Read for context (before acting):
  - AGENTS.md — repo conventions + hard safety rules (public repo, write containment)
  - .pipeline/option-close/PRD.md — what + D1-D8
  - .pipeline/option-close/arch.md — §Card 01 table + §Data flow steps 1-9 VERBATIM
  - .pipeline/option-close/CONTEXT.md + docs/adr/0022 — binding decisions
  - .pipeline/option-close/tasks/01.md then 02.md — the cards (oldest todo first)
Your task (concrete, numbered):
  1. Branch feat/option-close from trunk HEAD.
  2. Card 01: promote position_row to pub + 5 identity keys (arch §Card 01 exact mapping);
     re-export at ib root; verify: cargo build && cargo test --test positions_row green.
  3. Card 02: derive_close + shape_option_close_ack pure seams + option_close gateway fn
     (single-connect, steps in arch §Data flow order) + CLI/dispatch + docs amendment;
     verify: cargo test --test option_close_command green.
  4. Regression: cargo test --test stk_orders_command --test option_orders_command
     --test option_combo_command --test brief_command green; then FULL cargo test + clippy
     --all-targets -- -D warnings clean.
  5. wc -c CLAUDE.md < 900 BEFORE PR. Open ONE PR for the feature branch; flip both cards
     status: in-progress→review with your commits; journal seq=4; push.
Feature gotchas (project-specific traps the next node MUST know):
  - NEVER touch tests/positions_row.rs or tests/option_close_command.rs (frozen spec-paths;
    review diffs 286eb6a..tip over them — ANY change ⇒ reject).
  - SINGLE connect in option_close: drain, contract_details assert, place — all on the ONE
    client (a second same-client-id connect wedges the gateway; option-combo review lesson).
  - Validation ORDER frozen: usage < config(gate) < connection (dead-port matrix asserts it).
  - contract_details conid assert comes BEFORE place_with_client — refuse on mismatch (data).
  - right/security_type are non_exhaustive — always a _ fallback arm.
  - account_updates drain: reuse the End-marker pattern from positions(); LAST conid match wins.
  - CLAUDE.md is byte-budgeted (<900, frozen test) — the computed amendment is +15 bytes.
Done when: both cards' verify green + regressions + full suite + clippy; PR open; cards at
review; journal seq=4 pushed. On success: run pipeline-review. On failure: attempts++;
>=3 ⇒ blocked ⇒ run pipeline-hunt.
<<< END
