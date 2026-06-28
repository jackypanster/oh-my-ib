# review-02 — phase1-readonly / PR #1 addendum

Verdict: changes-requested (addendum to review-01)
Reviewer: Hermes gpt-5.5 + delegated Rust/CLI review fan-out
Time: 2026-06-28T09:19:06Z
PR: https://github.com/jackypanster/oh-my-ib/pull/1
Head reviewed by delegates: 9bd8e4e1ad262180f578aefc694c61eefd798e59

## Context

`review-01.md` already rejected PR #1 and routed cards 01+02 back to `todo` with `attempts: 1`.
The parallel delegated reviewers completed after that commit. This addendum records additional concrete
blockers discovered by that fan-out so the next `pipeline-impl` run has a complete fix list. This is the
same rejection event; do not increment attempts again for these addendum findings.

## Additional blockers

### BLOCKER A — live port can be selected without explicit `--live`

- Location: `src/config.rs:89-90`, `src/config.rs:112-113`
- Evidence: `CLAUDE.md` hard safety rule says paper account is default and live (`4001`) requires explicit `--live`. Current config merge accepts `port = 4001` from config or `--port 4001` directly.
- Trigger: `~/.config/oh-my-ib/config.toml` with `port = 4001`, or `omi account --port 4001`, connects to the live gateway without the explicit live opt-in flag.
- Required fix: make live selection structurally explicit. Reject effective port 4001 unless `--live` is present, or replace raw `--port` semantics with a safe enum/profile that cannot silently target live.

### BLOCKER B — `--account` / configured account is ignored

- Location: `src/ib/account.rs:18-20`, `src/ib/positions.rs:11-12`, `src/ib/orders.rs:10-12`
- Evidence: `Config.account` is populated but not used by account/positions/orders requests or output filtering. `account` uses group `All`; positions/orders read all accessible accounts.
- Trigger: in a multi-account paper setup, `omi --account DUxxx account|positions|orders` can still return other accessible accounts' data.
- Required fix: honor `cfg.account` for every account-scoped command. If an ibapi endpoint cannot filter by account, filter response rows by account and make the limitation explicit; otherwise route back to task for a spec adjustment.

### BLOCKER C — clap parse errors bypass the JSON error envelope

- Location: `src/main.rs:11-13`, `tests/cli_contract.rs:41-43`
- Evidence: `Cli::parse()` lets clap print its native text error and exit before `AppError` / `output::emit_error` can run. The frozen unknown-subcommand test only checks non-zero exit.
- Trigger: `omi frobnicate` exits with clap text stderr (`error: unrecognized subcommand...`), not `{"error":{...}}`.
- Required fix: use `Cli::try_parse()` or equivalent; preserve normal `--help`/`--version`, but convert parse failures into the structured error envelope.

### BLOCKER D — `--timeout` is a silent no-op

- Location: `src/cli.rs:40-42`, `src/config.rs:32-39`, `src/ib/mod.rs:28-30`
- Evidence: CLI accepts `--timeout`, but `Config` has no timeout field and `Client::connect` uses only address + client_id.
- Trigger: `omi health --timeout 1` behaves exactly like no timeout flag.
- Required fix: either implement timeout end-to-end (config + connect/request behavior) or remove the flag until a real timeout can be supported.

### BLOCKER E — config file uses `account`, not PRD `default_account`

- Location: `src/config.rs:53-60`, `src/config.rs:95-99`
- Evidence: PRD defines config field `default_account`; implementation deserializes `account` and serde ignores unknown `default_account` by default.
- Trigger: `~/.config/oh-my-ib/config.toml` with `default_account = "..."` leaves `Config.account = None`.
- Required fix: support `default_account` (and optionally alias `account`) and test config precedence for it.

### BLOCKER F — `orders` and `history` return Rust debug strings, not stable JSON objects

- Location: `src/ib/orders.rs:14-20`, `src/ib/history.rs:24`
- Evidence: JSON-first output contract requires agent-parseable data. Current `open_orders` and `bars` arrays contain `format!("{item:?}")` / `format!("{b:?}")` strings.
- Trigger: successful `omi orders` / `omi history` responses cannot be reliably parsed by field names and are unstable across Rust Debug formatting changes.
- Required fix: map each order/status/bar into explicit JSON objects with stable keys.

## Existing blockers from review-01 still apply

- Global `--md-type` is missing from `GlobalOpts` / config precedence.
- `account` output shape must be stable documented keys.
- `positions` output cannot satisfy `market_value` / `unrealized_pnl` as currently implemented; if ibapi `positions()` cannot supply them, route to `pipeline-task` for re-spec or choose a different endpoint.
