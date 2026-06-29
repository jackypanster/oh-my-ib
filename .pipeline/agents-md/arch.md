# arch — agents-md

Docs-only feature. One new tracked doc + one offline marker test. No `omi` code change.

## Pieces
1. `tests/agents_md.rs` (frozen spec, offline):
   ```
   let path = concat!(env!("CARGO_MANIFEST_DIR"), "/AGENTS.md");
   let text = std::fs::read_to_string(path).expect("AGENTS.md must exist at the repo root");
   for marker in ["agent-first", "Authoring — agent-first", "CONTRACT.md", "read-only", "OMI_ALLOW_LIVE"] {
       assert!(text.contains(marker), "AGENTS.md must contain {marker:?}");
   }
   assert!(text.len() > 800, "AGENTS.md must be substantive");
   ```
2. `AGENTS.md` (impl-owned; `git add` it). Canonical agent-conventions doc, sections:
   - One-paragraph project map: `omi` = read-only IBKR/Tiger CLI, agent-driven; `--live` (4001) /
     paper (4002); no API keys.
   - `## Authoring — agent-first` — the operator's rule: docs AND code for agents, not humans
     (the sentinel heading the test pins). Dense/structured docs; explicit types + stable JSON/exit-code
     interfaces; JSON output first, human/table secondary; "when unsure, assume agent".
   - `## Hard safety rules` — read-only Phase 1; public repo (no account ids/secrets); live needs
     `--live` and write needs `OMI_ALLOW_LIVE=1`; gateway timezone HKT auto-registered.
   - `## How this repo is built` — pipeline-driven; read `CONTRACT.md` in jackypanster/pipeline first,
     then `.pipeline/<feature>/` (PRD/arch/journal tail). Point at it; do not copy CONTRACT.md.

## Why the markers
The frozen test pins the load-bearing facts (agent-first rule present, pipeline pointer present, safety
facts present) without coupling to exact prose — impl may word the doc freely as long as the markers exist.

## Freeze coverage
Fully frozen + offline (a docs marker check). No gateway, no live acceptance. RED now because the
untracked AGENTS.md lacks the `agent-first` / authoring markers; green once impl writes + tracks AGENTS.md.

No ADR (docs/convention change).
