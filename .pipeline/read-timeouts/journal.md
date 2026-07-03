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

## seq=4 · 2026-07-03T07:10:00Z · impl→review · completed · by=oh-my-pi/glm-5.2
done:   impl landed on feat/read-timeouts (4 impl-paths: error.rs Timeout/6, ib/mod.rs const,
        two seam swaps verbatim per arch.md); card verify RED->GREEN (7), full suite GREEN
        (71), clippy clean; freeze-gate diff empty; PR #11 open.
output: PR https://github.com/jackypanster/oh-my-ib/pull/11,
        feat/read-timeouts commit e125825
--- handoff ---
>>> NEXT
Run pipeline-review on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=https://github.com/jackypanster/oh-my-ib/pull/11
Model: frontier SOTA required — operator assigns the bot; the pipeline can't verify the model.
First: git pull --rebase; no .env in this repo (CONTRACT step 2: nothing to load).
Read for context (before acting):
  - AGENTS.md + CLAUDE.md — repo conventions (public repo, read-only, agent-first docs)
  - .pipeline/read-timeouts/PRD.md — criteria 1-8, decisions D1-D5
  - .pipeline/read-timeouts/arch.md — §"Exact seam diffs" (the verbatim spec this impl followed)
  - .pipeline/read-timeouts/docs/adr/0012-take-first-timeout-twin.md — binding mechanism
  - .pipeline/read-timeouts/tasks/01.md — card 01 (status: review), §Freeze coverage
  - tests/read_timeouts.rs — the frozen spec (FROZEN — do NOT edit)
Your task (concrete, numbered):
  1. Freeze gate FIRST (deterministic): git diff 3b011a62aa9004c63c67cc0dc6c5b7c48103325c
     feat/read-timeouts -- tests/read_timeouts.rs must be EMPTY. Non-empty => reject
     (attempts++, route impl; >=3 => hunt).
  2. Full-suite gate on feat/read-timeouts HEAD: cargo build && cargo test (current.json
     full-verify) must be GREEN. Red => attribute to a card or write
     reviews/integration-NN.md + route hunt.
  3. Semantic review-by-reading (arch.md §Freeze coverage — NOT freezable offline, no fake IB
     server): both seams use timeout_iter_data(TAKE_FIRST_TIMEOUT).next() exactly (not raw
     next_timeout, not a local const); cure messages match arch.md verbatim (duration +
     restart cure; sweep arm prefixes "pnl_single conid {conid}:"); Some(Err) arms and all
     surrounding code byte-identical (PRD criterion 6 — healthy-path stdout unchanged); no
     other read site modified (drain loops keep iter_data()); error.rs envelope contract
     intact (code/exit table, Display shape); no new CLI/config surface; help text contains
     no "timeout".
  4. Operator live acceptance (PRD criterion 8, merge gate — NOT yours to run): omi --live pnl
     + omi --live brief healthy-path PASS in seconds. Flag this for the operator before merge.
  5. On all-green + human confirm: squash-merge PR #11 into main (only pipeline-review merges),
     flip card 01 status: review -> done, set current.json stage=done, append journal seq=5
     (metadata commit on main), push main.
Feature gotchas (project-specific traps the next node MUST know):
  - NEVER touch tests/ (frozen). NEVER run repo-wide cargo fmt — it rewrites tests/ (repo
    baseline is not fmt-clean).
  - The PR diff is ONLY the 4 impl-paths (error.rs, ib/mod.rs, ib/pnl.rs, ib/pnl_by_position.rs);
    metadata (card/journal/current.json) is on main, NOT in the PR.
  - reqPnL/reqPnLSingle are MARKERLESS (ADR 0007/0009): take-first only; the drain loops in
    account/positions/brief MUST keep iter_data() (this feature bounds ONLY the two take-first seams).
  - Exit 6 was free (1,2,3,4,5,64 taken); now taken by Timeout.
  - Public repo: no account ids/tokens/balances anywhere — recheck before merge.
Done when: PR #11 squash-merged into main after human confirm, card=done, journal seq=5 pushed.
On failure: attempts++; >=3 => blocked => run pipeline-hunt.
<<< END

## seq=5 · 2026-07-03T08:05:00Z · review→merge-confirm · completed · by=claude-fable-5
done:   pipeline-review APPROVE (review-01.md): freeze gate EMPTY; full suite 71/71 + clippy
        clean on detached worktree @ e125825; semantic pass verbatim vs arch.md; adversarial
        subagent 6/6 CONFIRMED; live: pnl PASS 2.9s (happy path un-slowed), timeout path proven
        LIVE 4x against a real wedge (bounded 10.2-10.3s, exit 6, exact envelope — pre-PR this
        hung forever). Side-finding: wedge strengthened to "first reqPnL per gateway session
        only" (8 data points, no kill needed); operational guidance = run brief first after
        gateway login. Optional non-blocking: brief first-slot healthy PASS at next restart.
output: .pipeline/read-timeouts/reviews/review-01.md
--- handoff ---
>>> NEXT
AWAITING OPERATOR: explicit merge confirmation for PR #11 (CONTRACT: only pipeline-review
merges, human-confirmed). On confirm, pipeline-review (any session) does:
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=https://github.com/jackypanster/oh-my-ib/pull/11
Model: frontier SOTA required — operator assigns the bot.
  1. Verify PR head still e125825 (gh pr view 11 --json headRefOid); if moved, re-run review gates.
  2. gh pr merge 11 --squash --delete-branch
  3. Verify trunk post-merge: cargo build && cargo test (expect 71/71) on updated main.
  4. Card 01 status review->done; current.json stage=done, pr stays for the record.
  5. Append journal seq=6 (feature DONE + open items: optional brief first-slot run;
     reqPnLSingle first-trading-day observation; gateway first-slot-only investigation).
  6. Single metadata commit on main, push.
Feature gotchas: none new — do not touch tests/; public repo redaction rules apply to any
pasted live output.
Done when: PR merged, trunk green, journal seq=6 pushed.
<<< END
