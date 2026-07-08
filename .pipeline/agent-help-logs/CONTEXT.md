# CONTEXT — agent-help-logs (domain language)

- **surface of record** — the `Command` enum in src/cli.rs is the ONLY authoritative list of
  what omi implements. Everything else (help registry, audit `cmd` names) must anchor to it
  via the staleness guards, never drift as a hand-maintained copy.
- **one-shot help** — `omi help` (no args): a single invocation returns the ENTIRE command
  surface as JSON. Exists so an agent never spends tokens on N+1 `--help` calls or reading
  src. `omi --help` (the clap flag) remains the terse human/builtin view.
- **write-gate marker** — the `gate` field on every help entry: `read-only` |
  `write` (paper default; live needs `--live` + `OMI_ALLOW_LIVE=1`) | `write-paper-only`
  (refuses the live port outright: grid-tick, sma-tick). Mirrors AGENTS.md hard safety rules.
- **invocation audit log** — append-only JSONL at `$HOME/.local/share/oh-my-ib/invocations.jsonl`;
  one line per parsed omi invocation (success AND failure), written at the dispatch seam.
  The local answer to "what did the agent do to the account?" and "did the monthly tick run?"
  (sma-monthly's omi calls land here automatically — zero strategy-lab coupling).
- **dispatch seam** — the single point in main() between `run(&cli)` returning and
  emit/exit, where the audit entry is written. One seam, no per-command instrumentation.
- **double staleness guard** — compile-time: `command_name()` exhaustive match (a new
  Command variant cannot build unnamed); runtime: set-equality test between clap subcommand
  names and help registry names. Together they make help provably complete.
- **fail-open audit** — an audit write failure warns on stderr (`warn:` plain line, NOT the
  JSON error envelope) and never changes the command's own result/exit (ADR 0037).
- **pre-config commands** — Help/Logs are handled BEFORE `Config::load()` in run(): they must
  work offline, with no gateway, and even with a missing/broken config.toml.
