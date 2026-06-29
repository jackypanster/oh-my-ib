# journal — agents-md

## seq=1 · 2026-06-29T15:51:11Z · prd→arch · completed · by=claude-opus-4-8(claude-code)
done:   New feature: commit AGENTS.md as the canonical agent-conventions doc + encode the operator's
        agent-first authoring principle (docs+code for agents, not humans). The untracked AGENTS.md is a
        near-copy of CLAUDE.md (onboarding only); it lacks the authoring rule. Frozen via an offline
        marker test that reads AGENTS.md. Decision-complete PRD written; current.json repointed.
output: .pipeline/agents-md/PRD.md, .pipeline/current.json
--- handoff ---
>>> NEXT
Run pipeline-arch then task then impl (rebuild from repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
Read: .pipeline/agents-md/PRD.md; existing untracked AGENTS.md; CLAUDE.md.
Design:
  - tests/agents_md.rs (offline): read concat!(env!("CARGO_MANIFEST_DIR"), "/AGENTS.md"); assert it
    contains markers: "agent-first", "Authoring — agent-first", "CONTRACT.md", "read-only", "OMI_ALLOW_LIVE".
    RED now (untracked AGENTS.md lacks "agent-first"/authoring section). impl writes AGENTS.md => green.
  - AGENTS.md (impl-owned, git-add it): canonical agent doc — agent-first authoring rules (with the
    sentinel heading "## Authoring — agent-first") + condensed project map + hard safety rules + pointer
    to jackypanster/pipeline CONTRACT.md + .pipeline/. Point at CONTRACT, do not copy it.
Gotcha: do NOT touch the other frozen specs (13e522d phase1, a072015 tz, fd72d90 connect-retry,
  cd191e2 quote). Do NOT modify CLAUDE.md (out of scope). This is docs-only — no live acceptance.
Done when: arch.md + freeze/record + impl + PR; merge human-confirmed.
On failure: attempts++; >=3 ⇒ blocked ⇒ pipeline-hunt.
<<< END

## seq=2 · 2026-06-29T15:52:47Z · arch→task · completed · by=claude-opus-4-8(claude-code)
done:   Locked design: tests/agents_md.rs reads AGENTS.md (CARGO_MANIFEST_DIR) and asserts markers
        [agent-first, "Authoring — agent-first", CONTRACT.md, read-only, OMI_ALLOW_LIVE] + len>800.
        AGENTS.md (impl-owned, git-add) = canonical agent doc: project map + agent-first authoring rule +
        safety rules + pipeline pointer. Markers pin facts without coupling to prose. No ADR.
output: .pipeline/agents-md/arch.md, .pipeline/current.json
--- handoff ---
>>> NEXT (pipeline-task)
First: git pull --rebase. Read .pipeline/agents-md/{PRD,arch}.md.
ONE card. Freeze tests/agents_md.rs (offline marker test, per arch.md). RED now (untracked AGENTS.md
  lacks markers). Two-commit freeze (test = new spec-rev; then card 01 + current.json stage=task).
Card: spec-paths=[tests/agents_md.rs], impl-paths=[AGENTS.md], verify=[cargo build, cargo test --test agents_md].
Gotcha: AGENTS.md is impl-owned (git add it). Do NOT touch other frozen specs or CLAUDE.md.
Done when: freeze+record on main; then pipeline-impl.
<<< END

## seq=3 · 2026-06-29T15:54:18Z · task→impl · completed · by=claude-opus-4-8(claude-code)
done:   Froze spec in two commits. Freeze 05d685f (tests/agents_md.rs only) = spec-rev; confirmed RED
        (clean compile+fail: untracked AGENTS.md lacks "agent-first"). Markers: agent-first,
        "Authoring (agent-first)", CONTRACT.md, OMI_ALLOW_LIVE (ASCII, robust). Record: tasks/01.md, stage=task.
output: .pipeline/agents-md/tasks/01.md, .pipeline/current.json
--- handoff ---
>>> NEXT (pipeline-impl; roles.yaml impl=goal-driven-impl-claude)
First: git pull --rebase. Read .pipeline/agents-md/{PRD,arch}.md + tasks/01.md.
  1. Branch feat/agents-md from main.
  2. Write AGENTS.md (per arch/card) and `git add` it. Must contain markers: agent-first,
     "Authoring (agent-first)", CONTRACT.md, OMI_ALLOW_LIVE; len>800.
  3. Green: cargo build + cargo test --test agents_md + full cargo test; clippy -D warnings; fmt
     (do NOT touch any tests/ spec file; do NOT touch CLAUDE.md).
  4. No live acceptance (docs only).
  5. Push feat/agents-md, open PR, flip card review + stage=impl + journal on main.
Gotcha: spec 05d685f is this feature's gate only; phase1/tz/connect-retry/quote specs stay frozen/untouched.
Done when: card review, PR open, stage=impl; then pipeline-review (human-confirmed merge).
<<< END

## seq=4 · 2026-06-29T15:57:37Z · impl→review · completed · by=claude-opus-4-8(claude-code)
done:   Implemented card 01 on feat/agents-md (commit 49b2db7). Authored + git-tracked AGENTS.md (3142
        bytes) as the canonical agent-first conventions doc; CLAUDE.md untouched. Frozen agents_md test
        green; all 5 freeze gates empty (05d685f/cd191e2/fd72d90/a072015/13e522d). build + clippy
        -D warnings + cargo test (31). Docs-only — no live acceptance. PR #5 opened. Card 01 → review.
output: feat/agents-md @ 49b2db7, PR https://github.com/jackypanster/oh-my-ib/pull/5,
        tasks/01.md (review), current.json (stage=impl, pr set)
--- handoff ---
>>> NEXT (pipeline-review; human-confirmed merge)
First: git pull --rebase. Read .pipeline/agents-md/{PRD,arch}.md + tasks/01.md; PR #5 diff.
  1. Freeze gates empty: git diff 05d685f origin/feat/agents-md -- tests/agents_md.rs; + inherited 4 specs.
  2. Full verify on origin/feat/agents-md: cargo build && cargo test GREEN; clippy --all-targets -D warnings.
  3. Read AGENTS.md: agent-first authoring rule present; safety facts; pipeline pointer (points at
     CONTRACT.md, not a copy); CLAUDE.md NOT modified.
  4. Docs-only — no live acceptance needed.
  5. HARD GATE: human-confirmed merge. On confirm: squash-merge PR #5, card→done, current.json.stage=done, journal.
On reject: attempts++; route to pipeline-impl; >=3 ⇒ blocked ⇒ pipeline-hunt.
<<< END

## seq=5 · 2026-06-29T16:06:06Z · impl→review · completed · by=codex-gpt-5(codex)
done:   Semantic review approved PR #5 with no blocking findings. Freeze gates were empty; final
        full-verify (`cargo build`, `cargo test`) ran green on isolated PR-head worktree 49b2db7;
        clippy `--all-targets -- -D warnings` also green. Review verdict written; awaiting human confirm.
output: .pipeline/agents-md/reviews/review-01.md, .pipeline/current.json
--- handoff ---
>>> NEXT
Run pipeline-review merge disposition on a FRESH session after explicit human confirmation.
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=https://github.com/jackypanster/oh-my-ib/pull/5
First: git pull --rebase; load repo config (.env if present, per CONTRACT step 2).
Read for context:
  - AGENTS.md or CLAUDE.md
  - .pipeline/agents-md/PRD.md
  - .pipeline/agents-md/arch.md
  - .pipeline/agents-md/tasks/01.md
  - .pipeline/agents-md/reviews/review-01.md
Your task:
  1. Proceed only after the operator explicitly says merge/go.
  2. Re-read PR #5 and confirm the head is still the reviewed commit 49b2db7 (or rerun freeze/full-verify if it moved).
  3. Confirm every agents-md card is still status=review.
  4. Squash-merge PR #5 via `gh pr merge --squash --delete-branch`.
  5. On main, set card 01 status=done, set current.json stage=done, append the final review→done journal entry, commit, and push.
Feature gotchas:
  - Never merge without explicit human confirmation.
  - PR patch is docs-only: it adds tracked AGENTS.md and does not touch CLAUDE.md or Rust code.
  - Freeze gates already ran empty against PR head 49b2db7; final full-verify already ran green on an
    isolated worktree in this review.
Done when: PR #5 squash-merged, card 01 done, current.json stage=done, final journal entry committed and pushed to main.
On failure: if rejected, attempts++ and card 01 -> todo (or blocked at >=3), then route pipeline-impl/hunt.
<<< END
