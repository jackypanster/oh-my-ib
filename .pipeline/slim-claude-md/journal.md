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

## seq=2 · 2026-06-29T16:20:06Z · arch→task · completed · by=claude-opus-4-8(claude-code)
done:   Locked design: tests/claude_md.rs reads CLAUDE.md and asserts contains("AGENTS.md") + 100<len<900.
        CLAUDE.md (impl-owned) slimmed to a pointer + one-line backstops (~400-700 bytes). No ADR.
output: .pipeline/slim-claude-md/arch.md, .pipeline/current.json
--- handoff ---
>>> NEXT (pipeline-task)
First: git pull --rebase. Read .pipeline/slim-claude-md/{PRD,arch}.md.
ONE card. Freeze tests/claude_md.rs (offline; per arch.md). RED now (CLAUDE.md 4316 bytes, no AGENTS.md
  ref). Two-commit freeze (test = new spec-rev; then card 01 + current.json stage=task).
Card: spec-paths=[tests/claude_md.rs], impl-paths=[CLAUDE.md], verify=[cargo build, cargo test --test claude_md].
Gotcha: CLAUDE.md is impl-owned. Do NOT touch AGENTS.md or other frozen specs.
Done when: freeze+record on main; then pipeline-impl.
<<< END

## seq=3 · 2026-06-29T16:22:26Z · task→impl · completed · by=claude-opus-4-8(claude-code)
done:   Froze spec in two commits. Freeze b61e9a5 (tests/claude_md.rs only) = spec-rev; confirmed RED
        (clean compile+fail: CLAUDE.md has no AGENTS.md ref). Record: tasks/01.md, current.json stage=task.
output: .pipeline/slim-claude-md/tasks/01.md, .pipeline/current.json
--- handoff ---
>>> NEXT (pipeline-impl; roles.yaml impl=goal-driven-impl-claude)
First: git pull --rebase. Read .pipeline/slim-claude-md/{PRD,arch}.md + tasks/01.md.
  1. Branch feat/slim-claude-md from main.
  2. Rewrite CLAUDE.md as a pointer at AGENTS.md + one-line backstops (must contain "AGENTS.md";
     100 < len < 900). Do NOT touch AGENTS.md.
  3. Green: cargo build + cargo test --test claude_md + full cargo test; clippy -D warnings; fmt
     (do NOT touch any tests/ spec file).
  4. No live acceptance (docs only).
  5. Push feat/slim-claude-md, open PR, flip card review + stage=impl + journal on main.
Gotcha: spec b61e9a5 is this feature's gate only; phase1/tz/connect-retry/quote/agents-md specs stay frozen.
Done when: card review, PR open, stage=impl; then pipeline-review (human-confirmed merge).
<<< END

## seq=4 · 2026-06-29T16:24:42Z · impl→review · completed · by=claude-opus-4-8(claude-code)
done:   Implemented card 01 on feat/slim-claude-md (commit 5cd155b). CLAUDE.md slimmed 4316→675 bytes,
        now a pointer at AGENTS.md + one-line backstops; AGENTS.md untouched. Frozen claude_md test green;
        all 6 freeze gates empty (b61e9a5/05d685f/cd191e2/fd72d90/a072015/13e522d). build + clippy
        -D warnings + cargo test (32). Docs-only — no live acceptance. PR #6 opened. Card 01 → review.
output: feat/slim-claude-md @ 5cd155b, PR https://github.com/jackypanster/oh-my-ib/pull/6,
        tasks/01.md (review), current.json (stage=impl, pr set)
--- handoff ---
>>> NEXT (pipeline-review; human-confirmed merge)
First: git pull --rebase. Read .pipeline/slim-claude-md/{PRD,arch}.md + tasks/01.md; PR #6 diff.
  1. Freeze gates empty: git diff b61e9a5 origin/feat/slim-claude-md -- tests/claude_md.rs; + inherited 5 specs.
  2. Full verify on origin/feat/slim-claude-md: cargo build && cargo test GREEN; clippy --all-targets -D warnings.
  3. Read CLAUDE.md: short pointer at AGENTS.md + backstops; AGENTS.md NOT modified; no Rust code touched.
  4. Docs-only — no live acceptance needed.
  5. HARD GATE: human-confirmed merge. On confirm: squash-merge PR #6, card→done, current.json.stage=done, journal.
On reject: attempts++; route to pipeline-impl; >=3 ⇒ blocked ⇒ pipeline-hunt.
<<< END
