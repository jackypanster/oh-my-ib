# Run journal — agent-help-logs

## seq=1 · 2026-07-08T07:09:55Z · ∅→prd · completed · by=cc/claude-fable-5
done:   PRD for agent-help-logs: `omi help` (one-shot agent-parseable command surface,
        staleness-proof vs the Command enum) + invocation audit JSONL at the dispatch
        seam + `omi logs` reader. Orders item DROPPED (already covered by
        orders/executions/completed-orders/brief). 3 human-confirmed + 5 code-verified
        decisions; 5 ⚠️ assumed rows tagged as mandatory arch challenge targets.
output: .pipeline/agent-help-logs/PRD.md
--- handoff ---
>>> NEXT
Run pipeline-arch on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
Model: frontier SOTA required — operator assigns the bot; the pipeline can't verify the model.
First: git pull --rebase; no .env in this repo (runtime config lives at ~/.config/oh-my-ib/config.toml — never commit it).
Read for context (before acting):
  - AGENTS.md — agent-first output/error contract + hard safety rules (read FIRST)
  - .pipeline/agent-help-logs/PRD.md — what (decisions are provenance-tagged)
  - src/cli.rs — the 25-command surface of record
  - src/main.rs — dispatch + clap error handling (the audit seam lands here)
  - src/config.rs — existing plain-HOME-join dirs convention (line 75)
  - docs/write-path-semantics.md — write-gate semantics help must surface
Your task (concrete, numbered):
  1. grill-with-docs the PRD against the codebase; every ⚠️ assumed row in PRD §Decisions
     is a MANDATORY challenge target (log path, JSONL schema, fail-open vs fast-fail,
     logs flags, help mechanism).
  2. Decide the help mechanism (clap introspection vs static table) with the staleness
     invariant (help inventory == Command enum) as the frozen test's hook.
  3. Decide the audit seam exactly (where in main.rs dispatch; what is redacted; failure
     behavior) — record irreversible/surprising choices as ADRs.
  4. Write .pipeline/agent-help-logs/arch.md + CONTEXT.md + docs/adr/*; set
     current.json.stage=arch; append your journal entry; ONE commit; push.
Feature gotchas (project-specific traps the next node MUST know):
  - clap's builtin `help` subcommand must be disabled/taken over (✅ the name stays
    `omi help`); keep main.rs's DisplayHelpOnMissingArgumentOrSubcommand behavior intact.
  - src/lib.rs EXISTS → frozen tests in tests/ can import modules (NOT the binary-only pitfall).
  - Public repo: NEVER log or commit credentials/account ids; fixtures must be synthetic.
  - Help must surface write gates (read-only | paper-default | live = --live + OMI_ALLOW_LIVE=1).
  - All behavior is local (no external API semantics) → no reference-behavior artifact gate expected.
Done when: arch.md + CONTEXT.md + ADRs landed, journal appended, pushed.
On success: stage=arch, then run pipeline-task.
On failure: attempts++; >=3 ⇒ blocked ⇒ run pipeline-hunt.
<<< END
