# review-02 — pcs-scan card 01 (round 2, delta 68ad974..7426cb1)

verdict: changes-requested
head: 7426cb1f84cd1d245d8a50b46a2bfe4ab7004ce9
prior: 68ad974f31da1aa3721b89d12c9c71adec4610f4 (review-01)
trunk: 77af4216f8b354ce9e7c7960ae3854a4f8e20e19
pr: https://github.com/jackypanster/oh-my-ib/pull/35
card: tasks/01.md status flipped review→todo, attempts 1→2
delta: src/ib/pcs_scan.rs only (+138/−17)
findings: 1 blocking (F2). review-01 F1's already-past-deadline buffer read is present.
merge: not performed (GO-gate not armed)

## Freeze gate

`git diff 4b19d3d2f94d7d8623a4b0809dd21f8ec19ce8b5 7426cb1 -- tests/pcs_scan.rs tests/help_command.rs` is empty.

## What the delta keeps

`drain_until_end` (`src/ib/pcs_scan.rs:815-837`) is the schedule. When the loop is entered with `remaining == 0`, it calls the non-blocking `buffered` closure (`try_next` in `drain_snapshot:847`), not `next_timeout(0)`. Test `drain_until_end_reads_buffered_items_after_deadline` reads Data+End from that path. Test `drain_until_end_past_deadline_empty_buffer_is_quote_error` returns `QuoteError` when that path's buffer is empty. Chunk drop-before-next is unchanged (`snapshot_batch:773-779`). That is the review-01 F1 entry condition (deadline already past before this subscription is polled).

## Finding F2

`src/ib/pcs_scan.rs:822-834` — a blocking read that waits out `remaining` and returns `None` is classified `Ok(())`.

`remaining` is captured before `blocking(remaining)` (`:822-826`). The `None` arm uses that captured value (`:833-834`): `None if remaining.is_zero()` ⇒ `QuoteError`, else `Ok(())`. It does not re-check `Instant::now() >= deadline` after the call, and it does not enter the buffered phase.

ADR 0038 classifies `None` after the read: `now >= deadline` ⇒ timeout, else the stream self-ended. 68ad974 had `None if Instant::now() >= deadline`. This delta dropped that post-call check. A `next_timeout` that blocks until the window ends and returns `None` (ibapi timeout, channel not read) therefore takes the `Ok(())` arm. `drain_snapshot` then returns `Ok(SnapData { ticks: empty, greeks: None })`.

Option pass: `ok_by_plan` increments on `Ok` (`pcs_scan.rs` pass-1 loop). An all-timeout pass 1 no longer hits `ok_by_plan == 0` ⇒ `quote_error`; `select_pcs` sees unusable rows and returns `no_greeks`. Stock pass still maps both `Err` and empty `Ok` through `result.ok().and_then(pick_spot)` to `spot_unavailable`. Empty rows are not usable legs (`bid > 0 && ask > 0`), so this does not emit a candidate. The skip reason and the ADR 0038 classification are wrong.

Test `drain_until_end_before_deadline_keeps_adr0038_semantics` (`:926-932`) returns `None` immediately, without sleeping for `remaining`, so the captured `remaining` is still non-zero for a true pre-deadline self-end. It does not cover a blocking timeout.

The card assumption added on this round ("empty channel stays `quote_error`"; the deadline is the cutoff) describes the already-past entry path. The blocking-timeout arm does not implement that cutoff.

### Probe (this session, not committed)

Worktree `/tmp/omi-pcs-r2` at 7426cb1. Temporary `#[test]` calling the real `drain_until_end`, then `git checkout` restored the file before full-verify:

```rust
let deadline = Instant::now() + Duration::from_millis(50);
drain_until_end::<u32>(deadline, |r| { std::thread::sleep(r); None }, || None, |_| {})
```

`cargo test --lib drain_until_end_blocking_none_at_deadline_is_quote_error` → FAILED.

```
assertion `left == right` failed
  left: Ok(())
 right: Err(QuoteError)
```

Required fix: after `blocking` returns `None`, if `Instant::now() >= deadline`, fall through to the buffered phase; `quote_error` only when that phase has no `SnapshotEnd`. A `None` that returns while `now < deadline` stays `Ok`. Add a regression test that sleeps for the passed `remaining` and then returns `None`. Do not edit spec-paths.

## Suite

full-verify on the restored 7426cb1 tree: `cargo test && cargo clippy --all-targets -- -D warnings` exit 0. The three new `drain_until_end` tests pass. They do not include the probe above. Clippy printed no warning.

## Live / OPRA

PRD criterion 7 stays deferred until OPRA, US RTH, `:4001`. Not this round's blocker. Coordinator note `/private/tmp/claude-501/-Users-user-workspace-oh-my-ib/33cd55bf-adb9-43f0-8829-83522af5faab/scratchpad/pr35-r2-coordinator-evidence.md` described this regression; the probe above is this session's run of that assertion, not a citation of their result as the verdict.

## doc debt

none.
