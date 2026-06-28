# review-03 — phase1-readonly / PR #1

Verdict: changes-requested
Reviewer: Codex gpt-5
Time: 2026-06-28T12:58:03Z
PR: https://github.com/jackypanster/oh-my-ib/pull/1
Head: 5545b5689e113f766ae8434c5d08c9fc62f9b6fa

## Deterministic gates

- Worktree preflight: `/Users/user/workspace/oh-my-ib` was clean on `main...origin/main`.
- PR state: `gh pr view 1` returned OPEN / MERGEABLE, head `5545b5689e113f766ae8434c5d08c9fc62f9b6fa`.
- Freeze gate: PASS. Ran two-commit diffs from shared spec-rev `13e522dc70a432b0403cd75d4b5b82531a77a6fa` to PR head for `tests/cli_contract.rs` and `tests/data_commands.rs`; both empty.
- Full verify on detached PR worktree `/tmp/oh-my-ib-pr1-review.smq1OT`: PASS for `cargo build`; PASS for `cargo test` (12 unit + 5 cli_contract + 7 data_commands + doctests); PASS for `cargo clippy --all-targets -- -D warnings`.
- Prior-blocker repros now pass: global `omi --md-type delayed quote AAPL --port 65000 --format json` reaches JSON connection envelope (exit 2); `omi frobnicate` emits JSON `usage` envelope (exit 64); `omi account --port 4001` emits JSON config error (exit 5).
- Read-only sweep: no write/order-placement sink found in `src` / `Cargo.toml` via `rg` for place/cancel/modify/transmit/submit-style terms.
- Secret sweep: no committed real account ids/tokens/secrets found; only test fake account strings and docs/config key names matched.

## Findings

### BLOCKER 1 — missing required subcommand exits successfully

- Location: `src/main.rs:17-21`
- Evidence: the PRD requires a non-interactive CLI with structured error envelopes and non-zero exits for errors. Review-02 also required clap parse failures to go through the JSON usage envelope while leaving explicit `--help` / `--version` unaffected. `DisplayHelpOnMissingArgumentOrSubcommand` is the clap error used when a required subcommand is missing, but the implementation groups it with `DisplayHelp` / `DisplayVersion` and exits 0.
- Trigger: on PR head 5545b56, `./target/debug/omi` exits 0 and prints help text. An agent or script checking only exit status will treat an invalid invocation as success.
- Required fix: preserve explicit `omi --help` and `omi --version` success, but convert missing required command into the structured usage envelope (`{"error":{"code":"usage",...}}`) and exit 64 (or at minimum non-zero per the error contract). If adding coverage, keep it in impl-owned source/unit tests; do not edit frozen `tests/cli_contract.rs` from impl.

## Non-blocking observations

- The prior Card 01/02 blockers are materially addressed: global `--md-type`, live-port guard, `default_account`, no-op `--timeout` removal, account/positions via `account_updates`, structured orders/history, and account filtering all checked by command output and/or code reading.
- Live value correctness remains unverified in this review session because no paper IB Gateway acceptance run was available; this is still a pre-merge human acceptance gate after semantic blockers are cleared.

## Disposition

Reject PR #1 for now. Route Card 01 back to `pipeline-impl` with `attempts: 2`; Card 02 can remain in `review`.
