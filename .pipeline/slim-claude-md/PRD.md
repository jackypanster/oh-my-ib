# PRD — slim-claude-md

Feature: slim `CLAUDE.md` to a thin pointer at the canonical `AGENTS.md`.

## Problem
`CLAUDE.md` (4316 bytes) duplicates `AGENTS.md` near-verbatim (project intro + safety rules + the full
pipeline onboarding block) and never references `AGENTS.md`. Two copies of the same conventions drift
apart and violate the agent-first single-source rule. `AGENTS.md` is now the tracked canonical doc.

## Goal
`CLAUDE.md` becomes a short pointer: "read `AGENTS.md` first", plus a few critical safety backstops
(public repo, read-only, live gate, pipeline). No duplicated onboarding block. `AGENTS.md` stays canonical.

## Success criteria
1. `tests/claude_md.rs` (offline) reads `CLAUDE.md` and asserts it references `AGENTS.md` and is short
   (100 < len < 900). RED until impl slims it.
2. `cargo build`, `cargo test`, `cargo clippy --all-targets -- -D warnings` green; all freeze gates empty.

## Scope
- `CLAUDE.md` (impl-owned): replace the body with a pointer at `AGENTS.md` + critical safety backstops.
- `tests/claude_md.rs` (frozen): the marker + length test.

## Non-scope
- No change to `AGENTS.md` (canonical) or any Rust code. No behavior change.

## Decisions
- `CLAUDE.md` keeps, inline, only the highest-value backstops (public repo / read-only / `--live` +
  `OMI_ALLOW_LIVE` / pipeline-driven), each one line, then points at `AGENTS.md` for full detail.
- Frozen markers: `CLAUDE.md` contains `AGENTS.md`; length 100 < len < 900 (enforces "stays a pointer").

## Freeze coverage
Frozen (`tests/claude_md.rs`, offline-deterministic): `CLAUDE.md` points at `AGENTS.md` and is short.
Fully offline — docs change, no gateway / live acceptance.
