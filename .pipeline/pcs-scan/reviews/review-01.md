# review-01 — pcs-scan card 01

verdict: changes-requested
head: 68ad974f31da1aa3721b89d12c9c71adec4610f4
merge-base: ffced4cb92f8f3842856bdd501dc4a14d4ed839f
trunk: 93cabd93302c77ff21613c49fa51da0147abdd6e
pr: https://github.com/jackypanster/oh-my-ib/pull/35
card: tasks/01.md status flipped review→todo, attempts 0→1
findings: 1 blocking
merge: not performed (GO-gate not armed)

## Freeze gate

`git diff 4b19d3d2f94d7d8623a4b0809dd21f8ec19ce8b5 68ad974f31da1aa3721b89d12c9c71adec4610f4 -- tests/pcs_scan.rs tests/help_command.rs` is empty.

## Scope

Three-dot `origin/main...68ad974` is exactly the impl-paths (+886/−1):

- `src/ib/pcs_scan.rs` (842 lines)
- `src/ib/mod.rs`
- `src/cli.rs`
- `src/main.rs`
- `src/surface.rs`

`src/ib/trade.rs` is not in the diff. `pcs_scan` does not call `trade`. Registry gate is `read-only` (`src/surface.rs:227-231`).

## Finding

`src/ib/pcs_scan.rs:772-805` — a silent head subscription in a chunk discards sibling snapshots that already completed inside the shared deadline.

`snapshot_batch` opens up to `PCS_BATCH_LINES` (50) subscriptions, then sets one `deadline = Instant::now() + SNAPSHOT_DEADLINE` (`:772`) and drains them in argv/chunk order (`:773-778`). `drain_snapshot` (`:790-805`) blocks in `next_timeout(remaining)` until SnapshotEnd, Err, or that deadline.

ibapi 3.1.0 `Subscription::next_timeout` (`subscriptions/sync.rs:222-231`) returns `None` immediately when the passed duration is zero, without `recv`. It does not flush a channel that already holds ticks or SnapshotEnd.

Trigger: the first contract of a chunk produces neither SnapshotEnd nor `Some(Err)` before the shared deadline (the ADR 0038 silent snapshot). That drain holds the thread until `now >= deadline`. Every later subscription in the same chunk then starts with `remaining == 0`, gets `None`, and is stored as `SkipReason::QuoteError` (`:803`). Messages those subscriptions buffered during the parallel window are dropped with the subscription. One silent contract therefore skip-fails the rest of the chunk. ADR 0039 §1 says a timeout is that contract's error and that the batch runs in parallel server-side so it costs one snapshot tail. The tail cost is one window; the read is not. A fast `Err` does not hit this — `ZZZZZZ`+`SPY` delayed (this session, 15.12s) left SPY as `quote_error`, so a head that returns before the deadline does not poison the sibling. The silent-head path is the one that does.

Required fix: a contract whose data arrived before the shared deadline must be read. A silent head must not classify unread siblings as timeout. Do not edit spec-paths.

## Axes that hold

1. Chunk size is `contracts.chunks(PCS_BATCH_LINES)` (`:761`). Subscribe failure is that slot's `QuoteError` (`:766`), not a command error. The chunk's `subs` vec is local; every subscription is dropped before the next chunk (`:773-779`). 20ms subscribe spacing matches the card assumption and arch blindspot (`:36`, `:768-770`). Per-contract `Err` does not fail the command (`pcs_scan` only skips). The gap is the sequential read above, not the cap, the skip-not-fail type, or the drop-before-next-chunk.
2. Gateway flow matches arch §1–7 aside from that read. `validate_params` runs before `connect` (`:516-518`). Spot pass is one `snapshot_batch` of `Contract::stock` (SMART/USD defaults). Chain step keeps SMART rows, prefers `trading_class == symbol`, else most expirations with lexicographic `trading_class` tie-break (`fetch_chain_info:712-737`), matching the card assumption. Pass 1 rows and pass 2 rows are appended into `rows_by_plan` before `select_pcs` (`:594-636`). Skip precedence is `spot_unavailable`, then chain `quote_error`, then `no_expiry_in_window`, then `quote_error` when `ok_by_plan == 0` (including the vacuous empty pass 1), then `select_pcs`'s `no_greeks` / `no_long_leg` / `below_min_credit` (`:540-657`, `:345-387`). `refine_strikes` receives the requested coarse strikes (`:609-614`), so a failed coarse quote is not retried.
3. Read-only holds. 842 lines are the frozen pure seams (`:42-470`), the gateway (`:478-808`), and three unit tests (`:810-842`). Every private function is called. No extra module layer. Size matches the card's mandated surface.

## Assumptions

All seven card assumptions match the diff. None contradict the spec. The sequential silent-head behavior is not one of them.

## Suite

full-verify on detached worktree `/tmp/omi-review-pcs` at 68ad974: `cargo build && cargo test && cargo clippy --all-targets -- -D warnings` exit 0. 40 targets, 339 passed, 0 failed. Clippy printed no warning. The frozen suite does not drive `snapshot_batch` (no-mock rule); green does not cover this finding.

## Live evidence (cited, not the verdict)

Coordinator summary: `/private/tmp/claude-501/-Users-user-workspace-oh-my-ib/33cd55bf-adb9-43f0-8829-83522af5faab/scratchpad/pr35-coordinator-evidence.md`. This session re-ran two read-only commands on the 68ad974 binary against `:4001` (no OPRA):

- `pcs-scan SPY --md-type delayed` → exit 0, 16.86s, one skip `quote_error` (spot worked; option leg failed)
- `pcs-scan ZZZZZZ SPY --md-type delayed` → exit 0, 15.12s, `ZZZZZZ` `spot_unavailable`, `SPY` `quote_error`

That second run shows a head which returns before the deadline does not poison SPY. It does not exercise a 20s silent head. The 38-symbol and live-NVDA rows in the coordinator file were not re-executed here. No leftover `omi`.

PRD criterion 7 (real quote numbers and greeks) stays deferred until the operator has OPRA, in US RTH, on `:4001`. Not a blocker for this verdict and not cleared by it.

## doc debt

none.
