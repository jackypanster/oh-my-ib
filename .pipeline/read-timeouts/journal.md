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

## seq=2 · 2026-07-03T06:40:00Z · arch→task · completed · by=claude-fable-5
done:   arch complete: 4-file diff pinned (error.rs Timeout/6, TAKE_FIRST_TIMEOUT const in
        ib/mod.rs, two seam swaps with verbatim diffs); ADR 0012 (timeout twin, None-collapse,
        per-item window); freeze-coverage split pinned for task. All ibapi claims source-verified.
output: .pipeline/read-timeouts/arch.md, .pipeline/read-timeouts/CONTEXT.md,
        .pipeline/read-timeouts/docs/adr/0012-take-first-timeout-twin.md
--- handoff ---
>>> NEXT
Run pipeline-task on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
Model: frontier SOTA required — operator assigns the bot; the pipeline can't verify the model.
First: git pull --rebase; no .env in this repo (CONTRACT step 2: nothing to load).
Read for context (before acting):
  - AGENTS.md + CLAUDE.md — repo conventions
  - .pipeline/read-timeouts/PRD.md — criteria 1-8, decisions D1-D5
  - .pipeline/read-timeouts/arch.md — the 4-file diff + §Freeze coverage (YOUR spec, pre-pinned)
  - .pipeline/read-timeouts/docs/adr/0012-take-first-timeout-twin.md — binding mechanism
  - tests/pnl_by_position_command.rs — the house freeze pattern to mirror (incl. house-red via
    unresolved imports; doc-comment style)
Your task (concrete, numbered):
  1. FREEZE COMMIT: write tests/read_timeouts.rs per arch.md §Freeze coverage — frozen surface:
     AppError::timeout code/exit/Display; full code<->exit table regression; TAKE_FIRST_TIMEOUT
     == 10s; omi --help surface unchanged; omi pnl dead-port still code=connection. ONE commit
     touching ONLY that file; must compile-fail (house-red: imports unresolved) => suite RED.
     Its hash = spec-rev.
  2. RECORD COMMIT: tasks/01.md frontmatter (status todo, attempts 0,
     verify=["cargo build","cargo test --test read_timeouts"],
     spec-paths=[tests/read_timeouts.rs],
     impl-paths=[src/error.rs, src/ib/mod.rs, src/ib/pnl.rs, src/ib/pnl_by_position.rs],
     spec-rev=<freeze hash>), card body pointing at arch.md verbatim diffs, ## Freeze coverage
     section (frozen vs review-by-reading vs live per arch.md), current.json stage=task +
     full-verify=["cargo build","cargo test"], journal seq=3. Metadata only — spec-paths
     untouched.
Feature gotchas (project-specific traps the next node MUST know):
  - Single card, single freeze commit (CONTRACT double-commit protocol) — spec-rev is commit #1's hash.
  - spec-paths ∩ impl-paths = ∅ — brief.rs/cli.rs/main.rs are NOT impl-paths (untouched by design).
  - NEVER run repo-wide cargo fmt (rewrites frozen tests/) — fmt src/** only.
  - Public repo: no account ids/tokens/balances anywhere.
  - Trunk goes RED at the freeze commit and stays red until impl merges — expected (no CI gate).
Done when: both commits pushed, card 01 status=todo, journal seq=3 appended.
On success: run pipeline-impl (operator will hand it to the oh-my-pi coding agent).
On failure: attempts++; >=3 => blocked => run pipeline-hunt.
<<< END

## seq=3 · 2026-07-03T06:52:00Z · task→impl · completed · by=claude-fable-5
done:   spec frozen: tests/read_timeouts.rs (7 tests, house-red via unresolved imports) @
        spec-rev 3b011a62aa9004c63c67cc0dc6c5b7c48103325c (freeze commit, spec-paths only);
        card 01 recorded (todo, verify card-scoped, spec/impl paths disjoint); full-verify set.
output: tests/read_timeouts.rs, .pipeline/read-timeouts/tasks/01.md
--- handoff ---
>>> NEXT
Run pipeline-impl on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
Model: capable-local OK (impl only) — operator assigns the bot; the pipeline can't verify the model.
First: git pull --rebase; no .env in this repo (CONTRACT step 2: nothing to load).
Read for context (before acting):
  - AGENTS.md + CLAUDE.md — repo conventions (public repo, read-only, agent-first docs)
  - .pipeline/read-timeouts/tasks/01.md — YOUR card: scope, hard constraints, freeze coverage
  - .pipeline/read-timeouts/arch.md — §"Exact seam diffs" is the verbatim implementation
  - .pipeline/read-timeouts/docs/adr/0012-take-first-timeout-twin.md — binding mechanism + rationale
  - .pipeline/read-timeouts/PRD.md + CONTEXT.md — criteria + glossary
  - tests/read_timeouts.rs — the FROZEN spec you must turn green (read-only for you!)
Your task (concrete, numbered):
  1. git checkout -b feat/read-timeouts (cut from current trunk main).
  2. Implement EXACTLY the card's impl-paths: src/error.rs (ErrorKind::Timeout, code "timeout",
     exit 6, AppError::timeout constructor), src/ib/mod.rs (pub const TAKE_FIRST_TIMEOUT = 10s),
     src/ib/pnl.rs + src/ib/pnl_by_position.rs (swap next_data() ->
     timeout_iter_data(super::TAKE_FIRST_TIMEOUT).next(); None arm -> AppError::timeout with the
     arch.md verbatim cure message; sweep arm keeps conid prefix). Follow arch.md §Exact seam
     diffs verbatim.
  3. Verify: cargo build && cargo test --test read_timeouts (card verify, must go green), then
     cargo test (full suite — single card, must be fully green) and
     cargo clippy --all-targets -- -D warnings.
  4. Self-check the freeze gate BEFORE committing:
     git diff 3b011a62aa9004c63c67cc0dc6c5b7c48103325c HEAD -- tests/read_timeouts.rs
     must print NOTHING (also: git status must show no tests/ changes).
  5. Commit the impl (conventional message, e.g. "feat(read-timeouts): bound take-first PnL
     reads with timeout twin (ADR 0012)"), push the branch, open the PR:
     gh pr create --base main --head feat/read-timeouts --title "feat(read-timeouts): bounded
     take-first reads (timeout twin, ADR 0012)" --body pointing at
     .pipeline/read-timeouts/{PRD.md,arch.md,docs/adr/0012-*.md} and card 01.
  6. Flip card 01 status: todo -> review (that one frontmatter line is in your write-set),
     set current.json stage=impl, append journal seq=4 (this file, append-only, one metadata
     commit on MAIN — not the feature branch), push main.
Feature gotchas (project-specific traps the next node MUST know):
  - NEVER touch tests/ (frozen). NEVER run repo-wide cargo fmt — it rewrites tests/ (repo
    baseline is not fmt-clean). fmt src/** only, or skip fmt.
  - The metadata commit (card status + current.json + journal) goes on MAIN; the code diff goes
    on feat/read-timeouts. Two different commits on two different refs.
  - Do NOT add a timeout flag/config key (help text must not contain "timeout" — frozen test).
  - Do NOT touch src/ib/brief.rs / src/cli.rs / src/main.rs / src/output.rs (out of impl-paths).
  - use super::TAKE_FIRST_TIMEOUT inside the seams (both files are ib/ submodules).
  - Public repo: no account ids/tokens/balances in code, comments, or the PR body.
Done when: PR open, card verify + full suite + clippy green on the branch, freeze-gate diff
empty, card=review, journal seq=4 pushed. On success: run pipeline-review.
On failure: attempts++; >=3 => blocked => run pipeline-hunt.
<<< END
