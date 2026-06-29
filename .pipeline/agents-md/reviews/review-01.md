# Review 01 - agents-md

Verdict: approve. No blocking findings.

Scope reviewed:
- PR #5 (`feat/agents-md` @ `49b2db7cdbf5cc1e7eb8e62d8f8a60cf5597aeaf`) via `gh pr diff 5 --patch`.
- Code/doc diff: `AGENTS.md` added only.
- Card: `.pipeline/agents-md/tasks/01.md`.

Semantic review:
- `AGENTS.md` is tracked by the PR and is 3142 bytes, satisfying the frozen substantive-doc requirement.
- The document includes the load-bearing markers: `agent-first`, `Authoring (agent-first)`, `CONTRACT.md`, and `OMI_ALLOW_LIVE`.
- It states the repo map, read-only phase, public-repo credential rule, paper/live port split, `--live` gate, future write gate, and gateway auth/no-API-key model.
- It points agents to the pipeline `CONTRACT.md` and `.pipeline/<feature>/` state bus without copying the contract body.
- `CLAUDE.md` is unchanged in the PR.

Advisory (non-blocking):
- `AGENTS.md:33` uses shorthand stage names in the flow (`pipeline-prd -> arch -> task -> impl -> review`). Since the next lines explicitly direct agents to `CONTRACT.md`, this is not a blocker. A future polish change could spell the command names as `pipeline-prd -> pipeline-arch -> pipeline-task -> pipeline-impl -> pipeline-review`.

Deterministic gates run in this review:
- Freeze gate: `git diff 05d685f155deef277f89678001ae72f1e798b01b 49b2db7cdbf5cc1e7eb8e62d8f8a60cf5597aeaf -- tests/agents_md.rs` -> empty.
- Inherited freeze checks also empty for `tests/quote_ticks.rs`, `tests/connect_retry.rs`, `tests/tz_aliases.rs`, `tests/cli_contract.rs`, and `tests/data_commands.rs`.
- `git diff --check 49b2db7cdbf5cc1e7eb8e62d8f8a60cf5597aeaf^ 49b2db7cdbf5cc1e7eb8e62d8f8a60cf5597aeaf` -> clean.
- Final full-verify on an isolated worktree at PR head: `cargo build` -> pass; `cargo test` -> pass (31 tests).
- Extra PRD gate: `cargo clippy --all-targets -- -D warnings` -> pass.

Notes:
- This is docs-only. No gateway/live acceptance is required.
