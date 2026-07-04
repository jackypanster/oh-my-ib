# journal — option-orders (append-only)

## seq=1 · 2026-07-04T00:56:32Z · prd→arch · completed · by=claude-fable-5
done:   PRD decision-complete under operator FULL-AUTO authority (2026-07-04 /think:
        "一路自动继续…merge之前才通知我" — stop only at merge gate or on blockers).
        option-buy/option-sell, LMT-ONLY v1 (D2, orchestrator call, operator may override),
        TIF=DAY, cancel REUSED, safety parity ADR 0017/0018 verbatim (zero new machinery),
        whole-contract qty + finite checks everywhere (review-01 lesson front-loaded),
        9-key option ack, place-core extraction (D7, stk frozen suite = regression net),
        validation reuse via pub(crate) (D6). Env unknown: paper options permission
        (criterion 10 observation, not impl failure).
output: .pipeline/option-orders/PRD.md
--- handoff ---
>>> NEXT
Run pipeline-arch (SAME orchestrator session continues under full-auto authority).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
Model: frontier SOTA required.
Read: PRD.md (criteria 1-10, D1-D8); src/ib/trade.rs (place core to extract, gate, allocator);
      src/ib/option_quote.rs (builder chain + normalize_right/parse_expiry to promote);
      .pipeline/stk-orders/docs/adr/0017+0018 (safety posture, verbatim parity).
Do: arch.md (component design: build_option_order + shape_option_order_ack + place-core
    extraction + OptionOrderArgs verbatim + docs amendment text), ADR 0020 (option write
    path: LMT-only + whole-contract qty + ack shape + core extraction), CONTEXT.md delta
    (marketable-LMT pattern, whole contracts), freeze coverage pin. Then journal seq=2.
<<< END

## seq=2 · 2026-07-04T00:58:09Z · arch→task · completed · by=claude-fable-5
done:   arch decision-complete (full-auto continues): build_option_order + 9-key
        shape_option_order_ack pure seams; place_core extraction (ack via closure — stk
        frozen suite = regression net); validation reuse via pub(crate) promotion of
        normalize_right/parse_expiry; OptionOrderArgs pinned verbatim; docs amendment
        text pinned; ADR 0020 (LMT-only, whole contracts, shared core, cancel reuse);
        ibapi facts re-verified (ExpirationDate Display zero-padded, OptionRight, builder
        chain). ONE card. Freeze coverage pinned incl. MANDATORY verbatim-compile pre-check
        (options-read seq=5 lesson).
output: .pipeline/option-orders/arch.md, CONTEXT.md, docs/adr/0020-option-single-leg-orders.md
--- handoff ---
>>> NEXT
Run pipeline-task (SAME orchestrator session, full-auto).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
Model: frontier SOTA required.
Do: ONE card (01). Red spec tests/option_orders_command.rs per arch §Freeze coverage.
    MANDATORY: verbatim-compile the spec with temp stubs BEFORE the freeze commit.
    Freeze commit (spec only) = spec-rev; record commit (card + current.json stage=task +
    full-verify + journal seq=3); push. Then dispatch pipeline-impl to π (omp pane).
<<< END

## seq=3 · 2026-07-04T01:00:56Z · task→impl · completed · by=claude-fable-5
done:   spec FROZEN: freeze commit 63f3232 (= spec-rev) carries card 01's red tests
        (tests/option_orders_command.rs, 22 tests; RED on unresolved oh_my_ib imports only;
        VERBATIM-compile pre-verified via temp stub target, deleted pre-commit — the seq=5
        options-read lesson is now house procedure). stk frozen suite re-checked green
        pre-freeze. ONE card at todo/0. Deliberate spec omission documented: no
        env+live+dead gate-pass twin (live-order hazard if :4001 up during test runs).
output: .pipeline/option-orders/tasks/01.md, tests/option_orders_command.rs (spec-rev 63f3232)
--- handoff ---
>>> NEXT
Run pipeline-impl on the omp pane (π / GLM-5.2), full-auto (orchestrator dispatches directly).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
Model: capable-local OK (impl).
First: git pull --rebase; no .env in this repo.
Read: CONTRACT.md; .pipeline/option-orders/tasks/01.md (THE card); arch.md (§Component design
      + §CLI + §Docs amendment VERBATIM); docs/adr/0020; CONTEXT.md; src/ib/trade.rs (the
      module you extend + refactor); src/ib/option_quote.rs (helpers to promote pub(crate)).
Do: card 01 → in-progress (metadata to main); branch feat/option-orders from trunk HEAD;
    implement per card §Scope; green: cargo build && cargo test --test option_orders_command;
    REGRESSION NET: cargo test --test stk_orders_command must stay green (place_core
    refactor byte-identity); pre-PR: FULL cargo test + clippy -D warnings; push; open PR
    (gh pr create, title 'feat(option-orders): option-buy/option-sell — card 01', body cites
    card + spec-rev 63f3232); card → review; journal seq=4; push metadata.
Gotchas: NEVER touch tests/**; freeze gate diffs 63f3232..tip over the spec; no repo-wide
    fmt; write calls only in trade.rs; validation<gate<connect ordering is frozen; LMT-only
    (no MKT arm); public repo no secrets.
Done when: verify green + stk suite green + full suite green + clippy clean + PR open +
    card review + seq=4 pushed. On failure: attempts++; >=3 ⇒ blocked ⇒ pipeline-hunt.
<<< END
