# review-01 — phase1-readonly / PR #1

Verdict: changes-requested
Reviewer: Hermes gpt-5.5
Time: 2026-06-28T09:14:58Z
PR: https://github.com/jackypanster/oh-my-ib/pull/1
Head: 9bd8e4e1ad262180f578aefc694c61eefd798e59
Base: origin/main @ b55f8a29da64881f29f56e7f1b9d340925475135

## Deterministic gates

- Worktree preflight: `git status --short --branch -uall` on `/Users/user/workspace/oh-my-ib` was clean on `main...origin/main`.
- Pulled/fetched: `git pull --rebase && git fetch origin main feat/phase1-readonly pull/1/head:refs/tmp/pr-1`.
- PR state: `gh pr view 1` returned OPEN, MERGEABLE, head `9bd8e4e1ad262180f578aefc694c61eefd798e59`.
- Freeze gate: PASS. Ran `git diff --exit-code 13e522dc70a432b0403cd75d4b5b82531a77a6fa 9bd8e4e1ad262180f578aefc694c61eefd798e59 -- tests/cli_contract.rs`, same for `tests/data_commands.rs`, and combined `-- tests/`; all empty.
- Full verify on detached PR worktree `/tmp/oh-my-ib-pr1`: PASS for `cargo build`; PASS for `cargo test` (7 unit + 5 cli_contract + 7 data_commands + doctests); PASS for `cargo clippy --all-targets -- -D warnings`.
- Extra Rust hygiene check: `cargo fmt --check` FAILS, but the diff is only formatting drift in frozen spec files under `tests/` that are already on trunk and were not touched by PR #1. Not treated as a PR #1 semantic blocker; fix only via task/re-freeze if the pipeline wants fmt as a frozen invariant.
- Read-only safety sweep: `grep -R "place_order\|cancel_order\|modify\|submit\|transmit" -n src Cargo.toml` returned no matches.
- Secret sweep: no committed account ids/tokens/secrets found in product source; matches were only docs/tests/config names.

## Findings

### BLOCKER 1 — global `--md-type` is specified but not implemented

- Location: `src/cli.rs:20-43`, `src/cli.rs:69-75`, `src/config.rs:104-122`
- Evidence: Card 01 and the PRD require `--md-type live|delayed|frozen` to be a global flag, parseable before or after subcommands, and config precedence `flag > toml > default`. The implementation exposes `md_type` only on `quote` (`QuoteArgs`) and `Config::merge_flags` never overlays it.
- Trigger: `./target/debug/omi --md-type delayed quote AAPL --port 65000 --format json` exits with clap error `unexpected argument '--md-type' found` and suggests only `quote --md-type`.
- Why existing tests miss it: the frozen test only checks that `quote --help` mentions `--md-type`, not the global placement/precedence contract.
- Required fix: add `md_type: Option<String>` to `GlobalOpts` with `global = true`, merge it into `Config::md_type`, and decide whether to keep/remove the quote-local duplicate so precedence is deterministic.

### BLOCKER 2 — `account` output does not match the acceptance JSON shape

- Location: `src/ib/account.rs:22-39`
- Evidence: PRD success criterion 2 and `arch.md:73-77` require stable fields such as `account`, `net_liquidation`, `total_cash`, `buying_power`, `available_funds`, and `currency`. The implementation returns `{ "account_summary": { "NetLiquidation": { "value", "currency", "account" }, ... } }` using raw IB tag names.
- Trigger: with a live paper gateway, an agent expecting the documented top-level snake_case fields will not find them and cannot reliably parse account state.
- Why existing tests miss it: no live gateway fixture validates value-shaping; ADR 0006 explicitly says output shaping must be read in review / paper accepted.
- Required fix: convert the requested account-summary tags into the documented stable JSON keys, include account/currency in stable fields, and handle missing required tags as `AppError::data` or clearly null fields according to the contract.

### BLOCKER 3 — `positions` omits required fields and uses the wrong quantity key

- Location: `src/ib/positions.rs:20-26`
- Evidence: PRD success criterion 3 and Card 02 require `[Position]` with `symbol`, `conid`, `qty`, `avg_cost`, `market_value`, and `unrealized_pnl`. The implementation emits only `symbol`, `conid`, `position`, and `average_cost`.
- Trigger: with any open position, the JSON lacks `qty`, `market_value`, and `unrealized_pnl`, so the Phase 1 paper acceptance criterion cannot pass as written.
- Why existing tests miss it: the frozen tests only cover help/error paths, not live position data shape.
- Required fix: emit the documented key names. If IB's `positions()` endpoint cannot provide market value / unrealized PnL directly, either join against a suitable account/portfolio endpoint or route back to `pipeline-task` to re-spec the acceptance shape honestly before merging.

## Advisory / follow-up (not the rejection reason)

- `src/ib/orders.rs:14-22` emits debug strings rather than structured order JSON. This was called out in the impl handoff as a known first cut; keep it as a follow-up unless paper acceptance requires structured fields now.
- `src/cli.rs:40-42` defines `--timeout`, but `Config` and `ib::connect` never use it. This makes the flag misleading and leaves live commands dependent on ibapi/OS blocking behavior. Fix with the blockers if cheap; otherwise make a follow-up card.
- `src/ib/quote.rs:76-84` / `src/ib/contract.rs:91-93` expose `sec_type`/`exchange`/`currency` style flags but the implementation builds `Contract::stock(symbol)` and ignores those inputs. Not blocking for the current stock-only AAPL acceptance path, but either honor or remove before broadening Phase 1 usage.

## Disposition

Reject PR #1 for now. The code is structurally read-only and the offline gates are green, but the live acceptance contract for the agent-facing JSON shape is not met. Route back to `pipeline-impl` for Card 01 (global md-type) and Card 02 (account/positions output), then re-run `cargo build`, `cargo test --test cli_contract`, `cargo test --test data_commands`, and the final `cargo build && cargo test` before review returns.
