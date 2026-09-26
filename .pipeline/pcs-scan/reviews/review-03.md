# review-03 — pcs-scan card 01 (round 3, delta 7426cb1..758a5b8)

verdict: approve
head: 758a5b8fdec2457a70fab3f58f39bfc2ae2d35a8
prior: 7426cb1f84cd1d245d8a50b46a2bfe4ab7004ce9 (review-02)
trunk: fa029d3d7fdbd589e461fe2e06687047adeeab96
pr: https://github.com/jackypanster/oh-my-ib/pull/35
card: tasks/01.md status=review attempts=2 (unchanged; done-flip waits for the human token)
delta: src/ib/pcs_scan.rs only (+63/−11)
findings: none blocking
merge: not performed (GO-gate armed in the reviewer session)

## Freeze gate

`git diff 4b19d3d2f94d7d8623a4b0809dd21f8ec19ce8b5 758a5b8 -- tests/pcs_scan.rs tests/help_command.rs` is empty.

## F2

`drain_until_end` (`src/ib/pcs_scan.rs:824-848`) classifies a blocking `None` after the call:

- `Instant::now() < deadline` ⇒ `Ok(())` (stream self-end, ADR 0016)
- otherwise the match arm is empty and the next iteration sees `remaining == 0`, so it reads only `buffered()` (`try_next`). `SnapshotEnd` ⇒ `Ok`. Buffer exhausted or `Error` ⇒ `QuoteError` (`:829-833`)

`remaining` is no longer the classifier. `now >= deadline` is the timeout side, matching ADR 0038.

## F1 still holds

The `remaining.is_zero()` branch calls `buffered()` only (`:826-834`). It does not call `blocking` / `next_timeout(0)`. `drain_until_end_reads_buffered_items_after_deadline` passed on 758a5b8. Chunk drop-before-next is outside this delta (`snapshot_batch` unchanged).

## Probes (this session)

On 758a5b8, inside `cargo test` (lib):

- `drain_until_end_blocking_none_at_deadline_is_quote_error` ok
- `drain_until_end_blocking_timeout_falls_through_to_buffer` ok

On 7426cb1, the same two tests added only in a temp worktree (not committed) both failed:

- sleep-out `None`, empty buffer: left `Ok(())`, right `Err(QuoteError)`
- sleep-out `None`, buffer `[Data(9), End]`: left `seen []`, right `[9]`

## Weekend live anomaly (not a blocker)

Coordinator note: `/private/tmp/claude-501/-Users-user-workspace-oh-my-ib/33cd55bf-adb9-43f0-8829-83522af5faab/scratchpad/pr35-r3-coordinator-evidence.md`. Not re-executed here. Reading:

- Stock skip `spot_unavailable` is `result.ok().and_then(pick_spot)` (`:541`). Both `Err` and `Ok` with no positive Last / bid+ask / Close land there. `pick_spot` accepts `DelayedClose` via `tick_price("Close")` (`:139-157`). A snapshot that actually carried DelayedClose 767.18 would not be `spot_unavailable`.
- The reported burst finished in 11.5–12.7s, under `SNAPSHOT_DEADLINE` (20s), so it did not take the new post-deadline arm. Pre-deadline `None` ⇒ `Ok` with whatever ticks were already seen is the same arm 68ad974 used (`None => break`). This delta does not add a stock path that returns empty `Ok` where 68ad974 returned `Err`.
- `no_greeks` requires `ok_by_plan > 0` (`:597`, `:643-656`) and then `select_pcs` finding no usable short delta. An option `Ok` with no bid/ask/delta (pre-deadline `None`, or a `SnapshotEnd` read from the F1 buffer) increments `ok_by_plan` and can replace an all-`Err` `quote_error`. That is the F1 read doing what review-01 asked, visible when a sibling `SnapshotEnd` is buffered and has no usable delta (no OPRA). It is not an F2 misclassification. The later paired A/B (both heads `quote_error`, 14.6–16.3s) matches that this did not stick.
- No code path in this delta explains a stable `spot_unavailable` regression against 68ad974. Left for operator RTH acceptance. Not a merge block.

PRD criterion 7 (real quotes and greeks) stays deferred until OPRA, US RTH, `:4001`.

## Suite

On `/tmp/omi-pcs-r3` at 758a5b8: `cargo build` exit 0, `cargo test` exit 0, `cargo clippy --all-targets -- -D warnings` exit 0. 40 targets. Lib tests 24 passed (the two new drain tests included). 0 failed. Clippy printed no warning.

## Assumptions

The round-2 assumption (empty channel past the deadline stays `quote_error`; the drain does not wait past it) matches this delta. No assumption contradicts the spec.

## doc debt

none.
