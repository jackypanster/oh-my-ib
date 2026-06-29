# journal — slim-claude-md

## seq=1 · 2026-06-29T16:20:06Z · prd→arch · completed · by=claude-opus-4-8(claude-code)
done:   New feature: slim CLAUDE.md (4316 bytes, duplicates AGENTS.md, never references it) to a thin
        pointer at the canonical AGENTS.md + critical safety backstops. Frozen via an offline marker +
        length test. Decision-complete PRD written; current.json repointed.
output: .pipeline/slim-claude-md/PRD.md, .pipeline/current.json
--- handoff ---
>>> NEXT
Run pipeline-arch then task then impl (rebuild from repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
Read: .pipeline/slim-claude-md/PRD.md; current CLAUDE.md; AGENTS.md.
Design:
  - tests/claude_md.rs (offline): read concat!(env!("CARGO_MANIFEST_DIR"), "/CLAUDE.md"); assert it
    contains "AGENTS.md" and 100 < len < 900. RED now (CLAUDE.md is 4316 bytes, no AGENTS.md mention).
  - CLAUDE.md (impl-owned): replace body with a pointer at AGENTS.md + one-line backstops (public repo,
    read-only, --live + OMI_ALLOW_LIVE, pipeline-driven). Do NOT touch AGENTS.md.
Gotcha: do NOT touch the other frozen specs (phase1/tz/connect-retry/quote/agents-md). Docs-only — no
  live acceptance. The threshold len<900 enforces "CLAUDE.md stays a pointer".
Done when: arch.md + freeze/record + impl + PR; merge human-confirmed.
On failure: attempts++; >=3 ⇒ blocked ⇒ pipeline-hunt.
<<< END
