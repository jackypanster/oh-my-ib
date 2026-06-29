# arch — slim-claude-md

Docs-only. One frozen test + a rewritten (slimmed) CLAUDE.md. No `omi` code change.

## Pieces
1. `tests/claude_md.rs` (frozen spec, offline):
   ```
   let path = concat!(env!("CARGO_MANIFEST_DIR"), "/CLAUDE.md");
   let text = std::fs::read_to_string(path).expect("CLAUDE.md must exist at the repo root");
   assert!(text.contains("AGENTS.md"), "CLAUDE.md must point at AGENTS.md");
   assert!(text.len() > 100 && text.len() < 900, "CLAUDE.md must be a short pointer (got {} bytes)", text.len());
   ```
2. `CLAUDE.md` (impl-owned): replace the 4316-byte body with a short pointer — "read AGENTS.md first" +
   one-line backstops (public repo / read-only / `--live` + `OMI_ALLOW_LIVE` / pipeline-driven, read
   `CONTRACT.md`). Target ~400-700 bytes.

## Why this shape
The frozen test pins the two load-bearing facts — CLAUDE.md references the canonical AGENTS.md, and it
stays short (a pointer, not a second copy) — without coupling to exact prose. A future legit need to
grow CLAUDE.md would re-freeze via pipeline-task.

## Freeze coverage
Fully frozen + offline (a docs marker + length check). No gateway / live acceptance. RED now (CLAUDE.md
is 4316 bytes and never says "AGENTS.md"); green once impl slims it. Do NOT touch AGENTS.md.

No ADR (docs/convention change).
