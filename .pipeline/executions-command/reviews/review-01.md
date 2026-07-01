# review-01 - executions-command

Verdict: approve; awaiting explicit human merge confirmation.

PR: https://github.com/jackypanster/oh-my-ib/pull/8
Head: 2e12854c5529a36a274dc600d307f66d0280636b
Review depth: quick (4 implementation files, +131/-0)
Scope: on target. The PR only adds the `omi executions` command wiring + gateway module named by card 01.

Findings:
- None blocking.

Source review:
- `src/ib/executions.rs:36-73` `merge_executions` (the frozen seam): indexes commissions by `exec_id`
  (last write wins), maps exec rows **in order**, joins the matching commission or emits three `null`
  fields when unmatched, drops orphan commissions (never iterated → no phantom row), returns
  `Value::Array`. `realized_pnl` runs through `super::pnl_number` (reuse, not reimplemented) → IB sentinel
  / non-finite / `None` → `null`. Matches the frozen contract exactly.
- `src/ib/executions.rs:75-127` `executions` (gateway): reuses `connect` + `resolve_account`; scopes
  server-side via `ExecutionFilter { account_code: account.0.clone(), ..Default::default() }`; **drains to
  End** with `for item in subscription.iter_data()` (ADR 0008 — NOT reqPnL take-first), mapping
  `ExecutionData → ExecRow` and `CommissionReport → CommissionRow`, then `merge_executions`. Emits
  `{ account, executions:[…] }`. Empty stream → `executions: []`, exit 0 (empty = success).
- ibapi field mapping verified against ibapi-3.1.0 `orders/mod.rs`: `Execution.{execution_id, order_id,
  perm_id:i64, time, exchange, side, shares, price, cumulative_quantity, average_price}`,
  `ExecutionSide::as_str() -> "BOT"/"SLD"`, `Contract.{symbol, contract_id}`, `CommissionReport.
  {execution_id, commission, currency, realized_pnl:Option<f64>}`. All names/types correct.
- `src/ib/mod.rs:16,26`: `mod executions;` + `pub use executions::{executions, merge_executions,
  CommissionRow, ExecRow};` — exports the seam + row types the frozen test constructs. `src/cli.rs:63-64`
  adds `Command::Executions` (no args). `src/main.rs:69` dispatches `Command::Executions =>
  ib::executions(&config)`. Unrelated commands untouched.
- `src/output.rs` untouched; `render_table` is generic over `Value`, so `--format table` is inherited
  (the `{account, executions:[…]}` shape renders like `positions`/`orders`; `null` renders as `null`).
- Read-only: no place/modify/cancel path, no `OMI_ALLOW_LIVE` write gate, no dependency-manifest change.

Key semantic invariant (ADR 0008) — verified by reading ibapi-3.1.0 source:
- `StreamDecoder<Executions>` (`orders/common/stream_decoders.rs:78`) maps `ExecutionDataEnd ->
  Err(Error::EndOfStream)`; the sync iterator (`subscriptions/sync.rs:171-173`)
  `ProcessingResult::EndOfStream => NextAction::Return(None)` — so `iter_data()` **terminates cleanly**
  (yields `None`), it does NOT surface `EndOfStream` as an `Err` that `item.map_err(..)?` would convert to
  a spurious `AppError`. The drain is correct and cannot hang (a terminator exists — the opposite of
  reqPnL/ADR 0007).
- `CommissionReport` (no request_id/order_id) is routed `ByExecutionId` (`transport/routing.rs:132`) via
  the exec_id→subscription mapping stored when the matching `ExecutionData` was routed (`routing.rs:129`),
  so commissions land in this same subscription and correlate by `exec_id`. Best-effort join by stream
  position: a commission arriving after `ExecutionDataEnd`, or never sent (Tiger gateway may omit/reorder),
  yields `null` commission fields — graceful, not an error. As designed.

Deterministic gates run in this review:
- PASS: freeze gate `git diff 374ea2f5c38f35e4222775da7f626ad73cefd2a5 2e12854c5529a36a274dc600d307f66d0280636b -- tests/executions_command.rs` -> empty (frozen spec untouched).
- PASS: no other frozen spec touched (`git diff --stat <spec-rev> <head> -- tests/` empty).
- PASS: PR surface from `gh pr diff 8` contains only `src/cli.rs`, `src/ib/executions.rs`, `src/ib/mod.rs`, `src/main.rs` (⊆ card impl-paths; `spec ∩ impl = ∅`).

Verification on `feat/executions-command` HEAD (2e12854):
- PASS: `cargo build`.
- PASS: `cargo test` (whole suite: 35 tests; includes `tests/executions_command.rs` 8/8; all sibling frozen specs green, no regressions).
- PASS: `cargo clippy --all-targets -- -D warnings`.

Live acceptance:
- Not run by this review session. Per the repo's gateway-dependent verification model, live
  `omi --live executions` acceptance is operator-run after the Tiger gateway reopens on `:4001`; not a
  merge blocker. Expectation: on a day with fills → itemized rows with `price` (+ `commission`/
  `realized_pnl` if the gateway sends them); `[]` on a flat day; corrections may appear as distinct rows.

Doc debt:
- None. The drain-to-End + best-effort-commission-join invariant is captured in ADR 0008, CONTEXT.md,
  the card, and this review record.

Merge gate:
- Do not merge until the operator gives explicit go.
