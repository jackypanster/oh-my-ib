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
