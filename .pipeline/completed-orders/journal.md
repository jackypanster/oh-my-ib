# journal — completed-orders (append-only)

## seq=1 · 2026-07-03T11:07:05Z · prd→arch · completed · by=claude-fable-5
done:   PRD decision-complete: omi completed-orders via reqCompletedOrders(api_only=false);
        operator locked D1 (feature over FX-quote/account-summary-tags) and D2 (team rotation:
        keep pi=impl, codex=review this round; paradigm = roles rotate, every stage SOTA,
        KB note 41.100). Code facts: CompletedOrdersEnd shared-channel marker = orders.rs
        drain-to-End verbatim; NOT ADR 0012 class.
output: .pipeline/completed-orders/PRD.md
--- handoff ---
>>> NEXT
Run pipeline-arch on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
Model: frontier SOTA required — operator assigns the bot.
First: git pull --rebase; no .env in this repo.
Read for context (before acting):
  - AGENTS.md + CLAUDE.md; .pipeline/completed-orders/PRD.md (D1-D6)
  - src/ib/orders.rs — the sibling to mirror VERBATIM (drain, filter-when-set, row style)
  - ~/.cargo/registry/src/*/ibapi-3.1.0/src/orders/ — OrderData/OrderState/Orders enum +
    completed_orders (sync.rs:108-115) + shared_channel_configuration.rs:45 (End marker)
Your task (concrete, numbered):
  1. Pin the completed-row key set from ibapi source: open-orders 10-key parity + which
     completion fields exist on OrderData/OrderState (status/completed_time/completed_status/
     filled quantity/avg price?) and their absent-value mapping (house null rules).
  2. Write arch.md: src/ib/completed_orders.rs component (pure row seam for the frozen spec +
     gateway drain fn), cli/mod/main wiring, --account filter parity with orders.rs.
  3. Write ADR 0015 (repo-global numbering: 0014 taken) — completed-orders read class
     (drain-to-End, api_only=false, OrderData-only arm), row-shape decision.
  4. CONTEXT.md — glossary deltas.
  5. Pin freeze coverage: frozen = pure row seam + CLI contract; review-by-reading = the
     drain fn (mirror check vs orders.rs); live = PRD criterion 8 (shape/exit on any day).
Feature gotchas (project-specific traps the next node MUST know):
  - READ-ONLY red line: no place/modify/cancel code anywhere (public repo, trading gated).
  - NEVER repo-wide cargo fmt; never touch tests/. Public repo: no account ids/secrets.
  - NOT ADR 0012's markerless class — no TAKE_FIRST_TIMEOUT.
  - brief is FROZEN at 8 top-level keys — do NOT add a section.
  - ADR numbering is repo-global — next is 0015.
Done when: arch.md + CONTEXT.md + docs/adr/0015-*.md committed (stage=arch, journal seq=2,
one commit, pushed). On success: run pipeline-task.
On failure: attempts++; >=3 => blocked => run pipeline-hunt.
<<< END
