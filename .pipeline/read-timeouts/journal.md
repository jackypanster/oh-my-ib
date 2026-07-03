# journal — read-timeouts (append-only)

## seq=1 · 2026-07-03T06:28:20Z · prd→arch · completed · by=claude-fable-5
done:   PRD decision-complete: bounded take-first reads (reqPnL/reqPnLSingle) via ADR 0007's
        recorded next_timeout fallback; operator locked timeout error code (=timeout/exit 6) and
        fixed 10s const; scope = the two shared seams covering all four call paths.
output: .pipeline/read-timeouts/PRD.md
--- handoff ---
>>> NEXT
Run pipeline-arch on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
Model: frontier SOTA required — operator assigns the bot; the pipeline can't verify the model.
First: git pull --rebase; no .env in this repo (CONTRACT step 2: nothing to load).
Read for context (before acting):
  - AGENTS.md + CLAUDE.md — repo conventions (agent-first docs, public repo, read-only, live gate)
  - .pipeline/read-timeouts/PRD.md — what + locked decisions D1-D5
  - .pipeline/pnl-command/docs/adr/0007-pnl-take-first-unbounded-stream.md — the recorded fallback this feature applies
  - .pipeline/brief-command/reviews/review-01.md §Live acceptance — the live wedge evidence
  - src/ib/pnl.rs (pnl_with_client) + src/ib/pnl_by_position.rs (sweep_pnl_singles) — the ONLY two seams to change
  - src/error.rs — ErrorKind/code/exit_code tables to extend
  - ~/.cargo/registry/src/*/ibapi-3.1.0/src/subscriptions/sync.rs:222-281 — next_timeout/timeout_iter_data (verify D1 yourself)
Your task (concrete, numbered):
  1. Verify D1's mechanism claim in ibapi-3.1.0 source: timeout_iter_data(d).next() is the
     notice-filtered timeout twin of next_data(); None on expiry AND on closed stream (instant).
  2. Write arch.md: exact seam diffs (both None arms -> AppError::timeout), where the shared 10s
     const lives, error.rs extension (Timeout kind, "timeout" code, exit 6, constructor).
  3. Write the ADR (repo-global numbering: next is 0012) under .pipeline/read-timeouts/docs/adr/
     recording the timeout-twin decision + the None-collapse consequence + per-item window caveat.
  4. CONTEXT.md: glossary for this feature (wedge, take-first, timeout twin) — brief-command's
     CONTEXT.md is the house style.
  5. Pin the freeze-coverage split for task: frozen = timeout<->6 envelope mapping + untouched
     sibling CLI contract; review-by-reading = the seam wiring (no fake IB server — no-mock rule,
     agent_docs/tests.md).
Feature gotchas (project-specific traps the next node MUST know):
  - NEVER run repo-wide cargo fmt (baseline not fmt-clean; it rewrites frozen tests/) — fmt src/** only.
  - Public repo: no account ids/tokens/balances in any committed artifact.
  - reqPnL/reqPnLSingle are MARKERLESS (ADR 0007/0009): take-first only, a drain loop hangs forever.
  - Success-path stdout must stay byte-identical on all four call paths (PRD criterion 6).
  - ADR numbering is repo-global across .pipeline/*/docs/adr/ — 0011 is taken, use 0012.
  - Exit code 6 is free today (1,2,3,4,5,64 taken — src/error.rs exit_code()).
Done when: arch.md + CONTEXT.md + docs/adr/0012-*.md committed (stage=arch, journal seq=2 appended,
one commit, pushed). On success: run pipeline-task.
On failure: attempts++; >=3 => blocked => run pipeline-hunt.
<<< END
