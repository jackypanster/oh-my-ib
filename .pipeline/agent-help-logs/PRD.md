# PRD — agent-help-logs

## Problem

omi is agent-driven (AGENTS.md), but its command surface is only discoverable via N+1
`clap --help` calls or reading `src/cli.rs` — token waste in every agent session.
And omi has ZERO runtime trace: an agent-driven TRADING CLI leaves no local record of
what was invoked (including `buy`/`sell`/`cancel`), so "did the monthly tick run?" and
"what did the agent do to the account?" are unanswerable from omi itself.
Evidence the discovery problem is real: the operator requested an orders-viewing command
without knowing `orders`/`executions`/`completed-orders` already exist.

## Goal

1. `omi help` — ONE invocation returns the complete implemented command surface,
   agent-parseable (JSON default).
2. Invocation audit log — every omi run appends one JSONL line at the dispatch seam;
   `omi logs` reads it back.

## Success criteria

- `omi help`: single call, no gateway needed (works offline), exit 0. Lists EVERY
  variant of `Command` (src/cli.rs) — staleness-proof: a test asserts the help
  inventory matches the enum, so a future command cannot be added without appearing
  in help. Per command: name, one-line purpose, args/flags, one usage example, and a
  write-gate marker (read-only | paper-default write | live needs `--live` +
  `OMI_ALLOW_LIVE=1`). JSON default; `--format table` renders a human view.
- Audit log: any `omi <cmd>` (success OR failure) appends exactly ONE JSONL entry;
  `omi logs` returns entries (newest last), `--tail N` limits; sma-monthly's omi
  invocations appear with ZERO changes to strategy-lab (they are just omi runs).
- Post-merge: `cargo build && cargo test && cargo clippy --all-targets -- -D warnings`
  all green (current full-verify).

## Scope

- New `Help` + `Logs` subcommands (src/cli.rs + handlers).
- ONE audit seam in src/main.rs dispatch (single write point after a command
  completes; never per-command instrumentation).
- Log location default following the repo's existing plain-HOME-join convention
  (src/config.rs:75); no new required config, no new dependencies unless arch
  justifies one.

## Non-scope

- Orders unified view (✅ human-confirmed DROPPED — `orders`/`executions`/
  `completed-orders`/`brief` already cover working/filled/cancelled; help exposes them).
- Reading external/automation log files (launchd sma-monthly out/err stay
  strategy-lab's concern).
- Gateway logs; log rotation/retention; log querying beyond `--tail`; any change to
  trade behavior.

## Decisions (provenance-tagged)

- ✅ human-confirmed: logs = omi-side invocation audit JSONL + `omi logs` reader
  (NOT a reader of external automation log files).
- ✅ human-confirmed: the orders item is dropped from this feature.
- ✅ human-confirmed: the command is named `omi help` (take over clap's builtin).
- 📖 code-verified: clap's builtin `help` subcommand exists today; a custom `Help`
  variant requires disabling/taking over the builtin. src/main.rs:23-24 special-cases
  `DisplayHelpOnMissingArgumentOrSubcommand` — that behavior must keep working.
- 📖 code-verified: 25 variants in `Command` (src/cli.rs:56-112) = the surface of
  record for the staleness test.
- 📖 code-verified: output contract = JSON default / `--format table`; errors to
  stderr as `{"error":{"code","message","context"}}` + non-zero exit (AGENTS.md).
- 📖 code-verified: src/lib.rs exists → frozen integration tests in `tests/` can
  import modules (binary-only pitfall does NOT apply).
- 📖 code-verified: write gates — paper `:4002` ungated; live requires `--live` AND
  `OMI_ALLOW_LIVE=1`; write code lives only in src/ib/trade.rs. Help MUST surface
  these markers per command.
- ⚠️ assumed (MANDATORY arch challenge targets):
  - Audit log path `~/.local/share/oh-my-ib/invocations.jsonl`, built by the same
    plain-HOME-join pattern as config.rs:75 (no dirs crate).
  - JSONL schema draft: `{ts, cmd, args, mode: "paper"|"live", preview: bool, exit,
    error_code?, duration_ms}` — arch hardens the field set; NEVER log tokens or
    credentials; consider omitting or truncating account ids even locally.
  - Log-write failure = fail-open (stderr warning, the command itself proceeds) —
    audit is observability, not the product; arch must challenge this against the
    operator's global fast-fail preference and record the resolution as an ADR.
  - `omi logs` v1 flags: `--tail N` (default 50) only.
  - Help content mechanism (clap introspection vs a static table in code) — arch
    decides; the staleness test is the invariant either way.
