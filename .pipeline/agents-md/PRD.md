# PRD — agents-md

Feature: track `AGENTS.md` as the repo's canonical agent-facing convention doc, and encode the
**agent-first** authoring principle in it.

## Problem
The operator's standing principle: this repo is agent/LLM-facing — docs AND code are written for an
agent to read and act on, not for human readability. That principle is currently only in the operator's
private memory, not in the repo, so a cold agent touching the repo would not know it. An `AGENTS.md`
exists in the working tree but is **untracked** and is just a near-verbatim copy of `CLAUDE.md`
(onboarding only); it does not state the agent-first authoring rule.

## Goal
Commit `AGENTS.md` as the canonical, version-controlled agent-conventions file. It must (a) declare the
**agent-first** authoring rule (docs + code for agents, not humans), (b) carry a condensed project map +
hard safety rules, and (c) point at the pipeline contract + `.pipeline/` state bus as the normative spec.
Any agent that reads `AGENTS.md` learns how to work in this repo.

## Success criteria
1. `tests/agents_md.rs` (offline) reads `AGENTS.md` from the crate root and asserts it contains the
   load-bearing markers: the literal `agent-first`, an authoring-rules marker, the `CONTRACT.md`
   pipeline pointer, and the read-only/public-repo safety facts. RED until impl writes them.
2. `AGENTS.md` is `git`-tracked on the feature branch.
3. `cargo build`, `cargo test`, `cargo clippy --all-targets -- -D warnings` green; all freeze gates empty.

## Scope
- `AGENTS.md` (tracked): canonical agent doc — agent-first authoring rules + condensed project map +
  hard safety rules + pointer to `jackypanster/pipeline` `CONTRACT.md` and `.pipeline/<feature>/`.
- `tests/agents_md.rs`: the frozen marker test.

## Non-scope
- Do NOT rewrite/slim `CLAUDE.md` (it keeps the pipeline onboarding; a later dedupe is a separate ask).
- No code/behavior change to `omi`.
- No exhaustive duplication of CONTRACT.md into AGENTS.md — point at it, don't copy it.

## Decisions
- `AGENTS.md` is the canonical agent-conventions file (the conventional name many runtimes read); it
  supersedes the untracked copy. Embed a stable sentinel heading `## Authoring — agent-first` so the
  frozen test can pin the section without coupling to prose.
- Agent-first authoring rule (the new content): docs dense/structured/self-contained/machine-parseable;
  code with clear boundaries, explicit types, stable JSON/exit-code interfaces; output agent-parseable
  first (JSON), human/table secondary; when unsure "human or agent?" assume agent.
- Frozen markers (must appear in AGENTS.md): `agent-first`, `Authoring — agent-first`, `CONTRACT.md`,
  `read-only`, `OMI_ALLOW_LIVE`.

## Freeze coverage
Frozen (`tests/agents_md.rs`, offline-deterministic): AGENTS.md exists and contains the markers above.
Fully offline — no gateway, no live acceptance needed (this is a docs/convention change).
