# journal — stk-orders (append-only)

## seq=1 · 2026-07-03T12:09:07Z · prd→arch · completed · by=claude-fable-5
done:   PHASE 2 OPENED (operator-authorized): first write path. PRD decision-complete:
        buy/sell/cancel verbs, LMT/MKT TIF=DAY, paper-free + live double-gate
        (--live AND OMI_ALLOW_LIVE=1, config error pre-connect, offline-frozen), bounded
        first-ack (shared 10s; timeout = UNKNOWN + never blind-retry), v1 = place+cancel
        (modify/notional-cap/dry-run explicitly deselected). Red-line docs amendment
        (AGENTS.md/CLAUDE.md) rides the PR. Acceptance on PAPER gateway (operator must log
        gateway into paper for criterion 11).
output: .pipeline/stk-orders/PRD.md
--- handoff ---
>>> NEXT
Run pipeline-arch on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
Model: frontier SOTA required — HIGHEST-STAKES feature to date (first write path); operator assigns.
First: git pull --rebase; no .env in this repo.
Read for context (before acting):
  - AGENTS.md + CLAUDE.md — the red line you are AMENDING (scope it precisely)
  - .pipeline/stk-orders/PRD.md — criteria 1-11, decisions D1-D5, non-scope list
  - ~/.cargo/registry/src/*/ibapi-3.1.0/src/orders/ — pin from source: place_order vs
    submit_order vs order builder; next_valid_order_id / order-id allocation; what the order
    subscription yields (OrderStatus/OpenOrder events — which is the FIRST ack); cancel_order
    call + its ack channel; routing domain (order-id domain per ADR 0010 table)
  - src/ib/pnl.rs + completed_orders.rs — bounded-wait house patterns (ADR 0012/0016)
  - src/config.rs — where the OMI_ALLOW_LIVE gate check lives (pre-connect, config-level)
Your task (concrete, numbered):
  1. Pin the ibapi write-call shapes from source (the PRD's ack design may reshape here —
     criteria stay). Decide place fn + first-ack event choice + cancel ack.
  2. Write arch.md: module split (place.rs or orders_write.rs), pure seams (order-building,
     ack-shaping — frozen-testable), gate-check helper placement, exact CLI arg structs,
     wiring; the AGENTS.md/CLAUDE.md amendment TEXT (verbatim, so impl copies it).
  3. Write ADR 0017 (numbering: 0016 taken) — the write-path safety architecture (double
     gate, bounded ack, no-retry, paper-first acceptance); note the review-polarity flip
     (writes must exist ONLY in the new module, unreachable from reads).
  4. CONTEXT.md — glossary (write path, double gate, first ack, UNKNOWN state).
  5. Pin freeze coverage: frozen = seams + gate matrix (3 verbs x missing-gate, offline) +
     arg validation + dead-port + help; review-by-reading = gateway fns + docs amendment;
     live = criterion 11 PAPER lifecycle.
Feature gotchas (project-specific traps the next node MUST know):
  - This PR may touch AGENTS.md/CLAUDE.md — normally forbidden territory; ONLY the red-line
    paragraph, verbatim per arch.md, nothing else.
  - Gate check MUST precede connect (frozen tests rely on it being offline-deterministic).
  - Reuse TAKE_FIRST_TIMEOUT (pub, src/ib/mod.rs) for every ack wait — no new consts.
  - NEVER repo-wide cargo fmt; never touch tests/; public repo: no account ids/secrets.
  - ADR numbering repo-global — next is 0017.
Done when: arch.md + CONTEXT.md + docs/adr/0017-*.md committed (stage=arch, journal seq=2,
one commit, pushed). On success: run pipeline-task.
On failure: attempts++; >=3 => blocked => run pipeline-hunt.
<<< END

## seq=2 · 2026-07-03T12:14:37Z · arch→task · completed · by=claude-fable-5
done:   arch complete: write-path safety architecture (ADR 0017 — containment in trade.rs,
        effective-port double gate pre-connect, order-id-first bounded ack via ADR 0016
        pattern, no-retry UNKNOWN-state posture); pure seams build_stk_order (ibapi Order
        fields, frozen) + shape_order_ack (6-key); gate matrix fully offline-frozen; docs
        amendment text pinned verbatim; ibapi write calls source-verified
        (place_order/cancel_order subscriptions, PlaceOrder/CancelOrder enums,
        next_valid_order_id).
output: .pipeline/stk-orders/arch.md, CONTEXT.md, docs/adr/0017-write-path-safety.md
--- handoff ---
>>> NEXT
Run pipeline-task on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
Model: frontier SOTA required — highest-stakes freeze so far.
First: git pull --rebase; no .env in this repo.
Read for context (before acting):
  - AGENTS.md + CLAUDE.md; .pipeline/stk-orders/{PRD.md,arch.md,docs/adr/0017-*.md,CONTEXT.md}
  - tests/quote_ticks.rs — precedent: frozen tests MAY use ibapi types (dev-dependency)
  - tests/completed_orders_command.rs — freshest house freeze style
Your task (concrete, numbered):
  1. FREEZE COMMIT: write tests/stk_orders_command.rs per arch.md §Freeze coverage —
     build_stk_order LMT/MKT x buy/sell (assert ibapi Order fields: action, total_quantity,
     order_type, limit_price, tif Day + Contract symbol/STK); shape_order_ack 6-key exact +
     MKT null limit; GATE MATRIX offline (3 verbs x: --live no-env => config; --port 4001
     no-env => config; --live + OMI_ALLOW_LIVE=1 + dead port => connection; paper default +
     dead port => connection) using assert_cmd .env()/.env_remove(); validation (qty<=0,
     --limit<=0, missing args => usage); --help lists buy/sell/cancel. ONE commit touching
     ONLY that file; house-red via use oh_my_ib::ib::{build_stk_order, shape_order_ack}.
     Hash = spec-rev.
  2. RECORD COMMIT: tasks/01.md (todo, attempts 0,
     verify=["cargo build","cargo test --test stk_orders_command"],
     spec-paths=[tests/stk_orders_command.rs],
     impl-paths=[src/cli.rs, src/ib/trade.rs, src/ib/mod.rs, src/main.rs, AGENTS.md, CLAUDE.md],
     spec-rev=<freeze hash>); card body: scope + hard constraints + freeze coverage per
     arch.md (incl. the docs-amendment-verbatim rule and the containment grep); current.json
     stage=task + full-verify; journal seq=3.
Feature gotchas:
  - Gate check precedes connect — the env-based tests are fully offline; use .env_remove to
    guarantee a clean env per test.
  - Frozen tests construct ibapi Order/Contract types directly (quote_ticks precedent).
  - impl-paths includes AGENTS.md/CLAUDE.md (docs amendment) — unusual but arch-pinned.
  - NEVER repo-wide cargo fmt; public repo: no secrets.
Done when: both commits pushed, card 01 todo, journal seq=3 appended.
On success: run pipeline-impl (operator hands to interactive pi/omp; codex reviews after).
On failure: attempts++; >=3 => blocked => run pipeline-hunt.
<<< END
