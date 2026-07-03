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
