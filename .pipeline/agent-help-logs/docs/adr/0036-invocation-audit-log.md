# ADR 0036 — invocation audit log: path, schema, redaction, v1 boundary

## Status
Accepted (arch, agent-help-logs).

## Decision

Every successfully PARSED omi invocation (success or failure alike) appends exactly one
JSON line to `$HOME/.local/share/oh-my-ib/invocations.jsonl`.

- **Path** — resolved like `Config::config_path()` (src/config.rs:73-76): plain
  `std::env::var_os("HOME")` + join, no dirs crate. `create_dir_all` the parent on first
  write. HOME-derived so tests override `HOME` and stay hermetic.
- **Schema** (stable keys; additive-only evolution):

```json
{"ts": "2026-07-08T07:20:00Z",        // RFC3339 UTC, `time` crate (existing dep)
 "cmd": "buy",                        // from surface::command_name() — the shared anchor
 "argv": ["buy","QQQM","10"],         // std::env::args().skip(1); --account value → "***"
 "mode": "paper",                     // "live" iff --live was passed, else "paper"
 "preview": false,                    // GlobalOpts.preview
 "exit": 0,                           // the process exit code about to be used
 "error": null,                       // AppError::code() string when failed, else null
 "duration_ms": 1234}
```

- **Redaction** — the argv value FOLLOWING `--account` (either `--account X` or
  `--account=X` form) is replaced with `"***"`. No credentials exist in argv by design
  (auth is the logged-in gateway), but account ids stay out of even local artifacts.
- **v1 boundary** — clap parse failures exit before a `Command` exists and are NOT
  audited. `omi help` / `omi logs` themselves ARE audited (uniform seam). No rotation,
  no retention policy v1 (monthly automation + ad-hoc agent use = trivial volume).

## Why

An agent-driven trading CLI must be able to answer "what was invoked, when, against
which mode, with what outcome" locally. One JSONL file + one seam is the smallest
mechanism that covers ad-hoc agent runs AND the launchd sma-monthly automation
(its omi calls are just invocations) without coupling omi to any external repo's
log layout.

## Consequences

- `omi logs` is a pure reader of this file (`--tail N`, default 50; skipped-malformed
  counted, never fatal).
- Schema keys are load-bearing for agents: additive evolution only; renames/removals
  need a new ADR.
- Parse-failure audit (argv exists, cmd does not) is an explicit non-goal v1; revisit
  only with evidence it matters.
