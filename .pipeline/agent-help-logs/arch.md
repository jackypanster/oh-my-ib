# arch — agent-help-logs

## Shape

Two agent-ergonomics commands + one audit seam. All local: no gateway, no config keys,
no new dependencies (`time` 0.3 + `serde_json` already present).

```text
src/cli.rs      Command += Help, Logs(LogsArgs); #[command(disable_help_subcommand = true)]
src/surface.rs  NEW — the "surface of record": command_name(&Command) exhaustive match,
                the HELP registry (purpose/usage/example/gate per command), help_json()
src/audit.rs    NEW — AuditEntry, log_path(), append(), read_tail(n)
src/main.rs     run(): Help/Logs handled BEFORE Config::load (must work with a missing or
                broken config file); main(): the audit seam wraps the outcome
src/output.rs   UNCHANGED — render_table() is a generic recursive Value renderer
                (verified src/output.rs:44-75), so help/logs get --format table for free
src/lib.rs      += pub mod surface; pub mod audit;
```

## Data flow — the audit seam

```text
main(): parse ok → started = Instant::now()
      → result = run(&cli)
      → exit = 0 | err.exit_code();  error = None | err.code()
      → audit::append(AuditEntry{…}) — FAIL-OPEN (ADR 0037): on write error print ONE
        plain-text `warn:` line to stderr; never the JSON envelope; never changes exit
      → emit_success / emit_error → process::exit    (unchanged tail)
```

Clap parse failures exit BEFORE a `Command` exists → NOT audited (v1 boundary, ADR 0036).
`omi help` and `omi logs` themselves ARE audited (uniform seam — no special cases).

## Resolutions of the PRD ⚠ rows (all code-verified; the human was not needed)

1. **Log path** — `$HOME/.local/share/oh-my-ib/invocations.jsonl`, resolved exactly like
   `Config::config_path()` (src/config.rs:73-76, `std::env::var_os("HOME")` + join). HOME-derived
   ⇒ black-box tests override `HOME` to a temp dir and stay hermetic. ADR 0036.
2. **Schema** — ADR 0036 (ts RFC3339 UTC via the existing `time` dep; `cmd` from
   `command_name()`; argv with `--account` value redacted).
3. **Fail-open on audit write failure** — ADR 0037.
4. **`omi logs` v1** — `--tail N` (default 50) only. Output
   `{"path": "...", "entries": [...], "skipped_malformed": n}`; newest last; missing file ⇒
   `entries: []`, exit 0; a malformed/truncated line is skipped AND counted, never a crash.
5. **Help mechanism** — static registry in surface.rs + a DOUBLE staleness guard:
   - compile-time: `command_name(&Command) -> &'static str` is an exhaustive match — adding a
     Command variant fails the build until it is named; the SAME fn feeds the audit `cmd` field,
     so both features share one anchor;
   - runtime test: set-equality between clap's subcommand names (`Cli::command().get_subcommands()`)
     and the help registry's names — a registry entry cannot go missing or dangle.
   Static entries carry what clap metadata cannot: the usage example and the write-gate marker.

## Help output contract (v1)

```json
{"global": {"flags": [{"name": "--format", "doc": "..."}, ...]},
 "commands": [{"name": "buy", "purpose": "...", "usage": "omi buy SYMBOL QTY [--limit P]",
               "example": "omi buy QQQM 10 --limit 55.50", "gate": "write"}, ...]}
```

- `gate` ∈ `read-only` | `write` (paper default; live = `--live` + `OMI_ALLOW_LIVE=1`) |
  `write-paper-only` (grid-tick, sma-tick — they refuse the live port).
- `omi help` takes NO arguments (one-shot IS the point).
- **Surprising consequence (accepted):** clap's builtin `omi help <cmd>` passthrough goes away
  with `disable_help_subcommand`; `omi help buy` now yields the usage error envelope. The
  `omi --help` FLAG path is untouched (main.rs:17-19 DisplayHelp arm), so the frozen
  `cli_contract.rs::help_lists_all_subcommands` stays green.

## Ownership

- surface.rs owns WHAT the surface is (names / examples / gates).
- audit.rs owns the JSONL file I/O (path, append, tail-read).
- main.rs owns WHEN (seam placement, fail-open application, pre-config routing).
- No `src/ib/*` changes; no config.rs changes.

## Test hooks (for pipeline-task)

Black-box per the tests/ convention (assert_cmd, predicates, offline, deterministic):

- help: `omi help` → exit 0, stdout parses as JSON, contains a known spread of commands
  (health/orders/buy/sma-tick), every entry has a `gate`, write commands marked.
- staleness: clap names == registry names (import `oh_my_ib::` — lib.rs exists; or expose via
  a `#[test]` beside surface.rs — task decides card scoping).
- audit + logs: `HOME=<tmp>` → run `omi health --port 1` (fails fast offline, exit 2) →
  `invocations.jsonl` has exactly 1 line (cmd=health, exit=2, error="connection") →
  `omi logs --tail 1` (same HOME) returns it under `entries` with `skipped_malformed: 0`.
- fail-open: `HOME=<tmp file, not dir>` (unwritable path) → command still succeeds/exits per
  its own result; stderr has a `warn:` line; stdout JSON unpolluted.
