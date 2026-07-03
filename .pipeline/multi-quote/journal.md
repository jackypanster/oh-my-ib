# journal — multi-quote (append-only)

## seq=1 · 2026-07-03T08:16:50Z · prd→arch · completed · by=claude-fable-5
done:   PRD decision-complete: variadic omi quote (N snapshots, one connection, input order);
        operator locked D1 (feature choice over search/completed-orders/scanner/news/chains),
        D2 (N=1 bare object byte-identical / N>=2 bare array), D3 (whole-command fail-fast,
        symbol named). Code facts verified: switch_market_data_type is per-connection,
        SnapshotEnd-bounded drains (NOT the ADR 0012 take-first class), frozen quote specs
        unaffected by variadic args.
output: .pipeline/multi-quote/PRD.md
--- handoff ---
>>> NEXT
Run pipeline-arch on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
Model: frontier SOTA required — operator assigns the bot; the pipeline can't verify the model.
First: git pull --rebase; no .env in this repo (CONTRACT step 2: nothing to load).
Read for context (before acting):
  - AGENTS.md + CLAUDE.md — repo conventions (agent-first docs, public repo, read-only, live gate)
  - .pipeline/multi-quote/PRD.md — what + locked decisions D1-D5
  - src/ib/quote.rs — the ONLY gateway module to change (seam extraction pattern: see how
    brief-command refactored pnl.rs into pnl_with_client — ADR 0011's shared-seam discipline)
  - src/cli.rs QuoteArgs — symbol: String -> symbols: Vec<String> (num_args(1..))
  - .pipeline/brief-command/docs/adr/0010-*.md — one-session sequential discipline (prior art)
  - tests/quote_ticks.rs + tests/data_commands.rs — frozen surfaces that must stay green
Your task (concrete, numbered):
  1. Verify in ibapi-3.1.0 source: snapshot subscription cleanup on drop (market_data/realtime/
     sync.rs) — that back-to-back snapshot requests on one session are clean (request-id domain,
     line released at SnapshotEnd/drop).
  2. Write arch.md: exact seam shape (quote_one taking &Client + shared args -> today's object),
     the N-shaping rule (1 => bare object, >=2 => Value::Array, input order), error-context
     scheme (quote/<symbol>), where the STK guard and md-type switch sit (once, pre-loop).
  3. Write ADR 0013 (repo-global numbering: 0012 taken) under .pipeline/multi-quote/docs/adr/
     recording batch-on-one-session + N-polymorphic output + fail-fast naming the symbol.
  4. CONTEXT.md: glossary deltas (batch snapshot, N-shaping, input-order rule).
  5. Pin freeze coverage for task: frozen = N-shaping seam (pure), zero-symbol usage error,
     dead-port envelope, existing quote specs untouched; review-by-reading = the gateway loop;
     live = criterion 9 (single + batch same-session cross-check).
Feature gotchas (project-specific traps the next node MUST know):
  - N=1 output must stay BYTE-IDENTICAL (existing agent flows + house red line) — the N-shaping
    seam must be pure and frozen-testable offline.
  - NEVER run repo-wide cargo fmt (rewrites frozen tests/) — fmt src/** only.
  - Public repo: no account ids/tokens/balances anywhere.
  - Snapshot drains are SnapshotEnd-BOUNDED — do NOT wrap them in ADR 0012 take-first timeouts
    (different stream class; PRD records the distinction).
  - ADR numbering is repo-global across .pipeline/*/docs/adr/ — next is 0013.
Done when: arch.md + CONTEXT.md + docs/adr/0013-*.md committed (stage=arch, journal seq=2,
one commit, pushed). On success: run pipeline-task.
On failure: attempts++; >=3 => blocked => run pipeline-hunt.
<<< END
