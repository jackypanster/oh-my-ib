# review-01 — snapshot-timeout card 01

verdict: approve
head: a32f1a5084e77766ae5967d4aec8273b1a95cce4
merge-base: af0d5cf1a2e0225fa388ec5a9776bf7f7f61849e
trunk: 8c6295d1efd9d9ecfcd68bf1e200c760dddc3521
pr: https://github.com/jackypanster/oh-my-ib/pull/34
card: .pipeline/snapshot-timeout/tasks/01.md status=review attempts=0 spec-rev=75ae0cfd4839bffb715226e89f3dddfa0a2eb040
findings: none
merge: not performed (GO-gate armed in the reviewer session)

## Freeze gate

`git diff 75ae0cfd4839bffb715226e89f3dddfa0a2eb040 a32f1a5084e77766ae5967d4aec8273b1a95cce4 -- tests/snapshot_timeout.rs` is empty.
Three-dot `origin/main...a32f1a5` touches no other `tests/*`.

## Scope

Three-dot diff is exactly the card impl-paths (+120/−43):

- `src/error.rs`
- `src/ib/mod.rs`
- `src/ib/option_quote.rs`
- `src/ib/quote.rs`

The feature-branch tree still carries the pre-impl card (`status: todo`, no `## Assumptions`). That file is absent from the three-dot diff. `git merge-tree` of origin/main and a32f1a5 reported no conflict. Squash-merge keeps trunk's card (`status: review` + Assumptions).

## Assumptions

`tasks/01.md` `## Assumptions` is present. `quote_one`'s `md_type: &str` is the last parameter (`src/ib/quote.rs:72`). The only caller is `quote()` (`src/ib/quote.rs:49`). `pub(crate)`. Does not contradict the card.

## Axis 1 — total deadline, both drains

`SNAPSHOT_DEADLINE` is `Duration::from_secs(20)` (`src/ib/mod.rs:68`). The `20` in the message is `SNAPSHOT_DEADLINE.as_secs()` (`src/ib/mod.rs:79`). Neither drain calls `timeout_iter_data` or `TAKE_FIRST_TIMEOUT`.

`quote_one` (`src/ib/quote.rs:89-115`) and `option_quote` (`src/ib/option_quote.rs:219-248`) both do:

- `deadline = Instant::now() + SNAPSHOT_DEADLINE` immediately after `subscribe()`
- `remaining = deadline.saturating_duration_since(Instant::now())`
- `subscription.next_timeout(remaining)`
- `SubscriptionItem::Notice` => `continue`
- `None if Instant::now() >= deadline` => `snapshot_timeout_error`
- `None` => `break` with ticks already gathered

ibapi 3.1.0 `Subscription::next_timeout` (`subscriptions/sync.rs:222-236`) builds its own `Instant` deadline from the duration it was given and returns `None` once that remaining is zero. That inner deadline is at or after the caller's deadline, so a timeout `None` is classified as timeout. An end-of-stream `None` before the deadline breaks with the ticks gathered (ADR 0038 blindspot 2, same as the old `iter_data` end).

## Axis 2 — drop before the next symbol

`quote` fail-fasts with `?` (`src/ib/quote.rs:49-56`). `subscription` is a local of `quote_one`. `return Err` drops it before `quote` starts the next symbol. `Drop` calls `cancel()` (`subscriptions/sync.rs:284-289`). `cancel` returns immediately only when `snapshot_ended` is set (`sync.rs:78-81`), and that flag is set only on `TickTypes::SnapshotEnd` (`sync.rs:162-164`). A timeout never observes `SnapshotEnd`, so `TickTypes::cancel_message` encodes `CancelMarketData` (`market_data/realtime/mod.rs:377-379`). `option-quote` is one contract; the same drop runs on its `Err` return. `.snapshot()` exists only at these two call sites.

## Axis 3 — unchanged arms and success JSON

`Some(Err)` is still `AppError::data(format!("market_data stream: {e}"), ctx)` with `quote/{symbol}` (`src/ib/quote.rs:101-106`) and `option-quote` (`src/ib/option_quote.rs:235-240`).

Pre-connect order is unchanged. `quote` rejects a non-STK `--sec-type` before `connect` (`src/ib/quote.rs:19-25`). `option_quote` rejects right, strike, and expiry before `connect` (`src/ib/option_quote.rs:155-181`).

Success output is unchanged. `quote_one` still returns `{symbol, delayed, ticks}` (`src/ib/quote.rs:118-122`). `shape_quotes` is not in the diff (ADR 0013 N=1 bare object). `option_quote` still calls `shape_option_quote` with the same arguments (`src/ib/option_quote.rs:252-263`). Per-tick handling is still `quote_price_tick` plus last-model-row-wins greeks. `ErrorKind::Timeout` doc only (`src/error.rs:10`); code and exit 6 are untouched.

Instrument label for option timeout is `format!("{} {} {} {}", symbol, expiry, strike, normalized)` (`src/ib/option_quote.rs:218`). Rust `Display` of `225.0f64` is `225`, of `182.5f64` is `182.5` (checked with `rustc` this session). Success JSON still echoes `args.right`, not this label.

## Pre-merge guards

- Cards: 1/1 `status: review`.
- full-verify on detached worktree `/tmp/omi-review-a32f1a5` at a32f1a5: `cargo build && cargo test && cargo clippy --all-targets -- -D warnings` exit 0. 39 test targets, 320 passed, 0 failed. Clippy printed no warning and no error. `tests/snapshot_timeout.rs` 7/7 passed inside that run.

## Live evidence (cited, not the verdict)

Coordinator summary: `/private/tmp/claude-501/-Users-user-workspace-oh-my-ib/33cd55bf-adb9-43f0-8829-83522af5faab/scratchpad/pr34-coordinator-evidence.md`. This session re-ran the same reads against Gateway `:4001` with `target/debug/omi` built from a32f1a5. No raw coordinator transcript was in that scratchpad; the numbers below are this session's.

- `omi --format json --live --md-type live quote NVDA` → exit 6, 20.04s, stdout empty, stderr `code=timeout` `context=quote/NVDA` message `no SnapshotEnd within 20s for NVDA (md-type=live) — no real-time snapshot entitlement for this instrument, or market closed with no ticks; try --md-type delayed`
- `omi --format json --live --md-type live quote AAPL SPY` → exit 6, 20.02s, stdout empty, `context=quote/AAPL` (fail-fast; SPY not reached)
- `omi --format json --live --md-type delayed quote SPY` → exit 0, 12.75s, bare object `delayed=true` `symbol=SPY` with DelayedClose/High/Low/Open ticks
- `omi --format json --live --md-type live option-quote --symbol NVDA --expiry 20261016 --strike 180 --right C` → exit 4, 2.49s, `code=data` `context=option-quote` message `market_data stream: [10091] …`
- `pgrep -x omi` after the four commands: no process

## Sibling sweep

Snapshot drains: 2 sites, both swapped. Other `iter_data` users (account, positions, orders, executions, brief, pnl-by-position) are different stream classes, already self-ending or already on `TAKE_FIRST_TIMEOUT`. Out of card scope.

## doc debt

none. The bound is ADR 0038. No new write gate.
