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
