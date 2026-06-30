# review-01 - pnl-command

Verdict: approve; awaiting explicit human merge confirmation.

PR: https://github.com/jackypanster/oh-my-ib/pull/7
Head: 38d0cbfcac9af7267ee1f555627ed0a7a5e2922d
Review depth: quick (4 implementation files, +48/-0)
Scope: on target. The PR only adds the `omi pnl` command wiring and gateway module named by card 01.

Findings:
- None blocking.

Source review:
- `src/ib/pnl.rs:13-18`: reuses `connect` and `resolve_account`, then calls `client.pnl(&account, None)` for account-level PnL.
- `src/ib/pnl.rs:20-25`: takes exactly one item with `subscription.next_data()` and handles `Ok`, stream error, and closed-stream cases with structured `AppError::data`.
- `src/ib/pnl.rs:27-32`: emits the required JSON object: `account`, `daily_pnl`, `unrealized_pnl`, `realized_pnl`.
- `src/ib/pnl.rs:38-42`: `pnl_number` maps finite non-sentinel values to JSON numbers and maps `None`, non-finite values, and `f64::MAX`/`-f64::MAX` to `null`.
- `src/ib/mod.rs:18,27`, `src/cli.rs:57-58`, and `src/main.rs:63-67`: the module, public seam export, CLI variant, and dispatch are wired without changing unrelated commands.
- `src/output.rs` is untouched; `render_table` is generic over `serde_json::Value`, so `--format table` remains inherited behavior.
- No order-placement/modify/cancel path is introduced, no `OMI_ALLOW_LIVE` write gate is added, and no dependency manifest changes appear in the PR diff.
- Local ibapi 3.1.0 source confirms sync `Client::pnl(&AccountId, Option<&ModelCode>) -> Subscription<PnL>`, `PnL { daily_pnl: f64, unrealized_pnl: Option<f64>, realized_pnl: Option<f64> }`, `Subscription::next_data()` filters notices, and `Drop for Subscription` cancels the request.

Deterministic gates run in this review:
- PASS: freeze gate `git diff --exit-code dc9357de41ddfe7bdd4dd74a5eff979c04ff3986 38d0cbfcac9af7267ee1f555627ed0a7a5e2922d -- tests/pnl_command.rs` -> empty.
- PASS: `git diff --check origin/main...refs/tmp/pr-7`.
- PASS: PR surface from `gh pr diff 7` contains only `src/cli.rs`, `src/ib/mod.rs`, `src/ib/pnl.rs`, and `src/main.rs`.

Verification on detached PR-head worktree `/tmp/omi-pr7-review.jTvb9M/wt`:
- PASS: `cargo build`.
- PASS: `cargo test` (38 tests; includes `tests/pnl_command.rs` 6/6).
- PASS: `cargo clippy --all-targets -- -D warnings`.

Live acceptance:
- Not run by this review session. The pipeline states live `omi --live pnl` acceptance is operator-run after the Tiger gateway reopens on `:4001`; this is not a merge blocker under the repo's gateway-dependent verification model.

Doc debt:
- None. The take-first stream invariant is already captured in ADR 0007, the feature context, the card, and this review record.

Merge gate:
- Do not merge until the operator gives explicit go.
- Before merging, re-read PR head. If it changed from `38d0cbfcac9af7267ee1f555627ed0a7a5e2922d`, rerun review, freeze gate, and `current.json.full-verify` before merging.
