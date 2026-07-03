# review-01 — read-timeouts (PR #11, card 01)

Verdict: **APPROVE** — awaiting explicit operator merge confirmation. All deterministic gates
green; semantic + adversarial review clean; live acceptance PASSED for `pnl` and — bonus far
beyond the plan — the timeout path itself was proven LIVE 4× against a real gateway wedge
(pre-PR behavior: infinite hang). One optional strengthening run is recorded below (§Live,
brief first-slot) — the operator may run it at the next gateway restart, before or after merge.

Reviewed head: `e125825` (feat/read-timeouts). Spec-rev: `3b011a6`.

## Deterministic gates

- **Freeze gate: EMPTY.** `git diff 3b011a6..e125825 -- tests/read_timeouts.rs` = 0 lines;
  whole `tests/` vs main = 0 lines.
- **Scope exact:** merge-base three-dot diff = the card's 4 impl-paths only (+37 −13);
  `brief.rs`/`cli.rs`/`main.rs`/`output.rs`/drain loops untouched.
- **Full-suite gate: GREEN** on a detached worktree at `e125825`: `cargo build` clean,
  `cargo test` **71 passed / 0 failed** (frozen spec `read_timeouts` 7/7 red→green),
  `cargo clippy --all-targets -- -D warnings` clean.

## Semantic review (line-by-line vs arch.md + independent adversarial pass)

Direct read: diff matches arch.md §"Exact seam diffs" verbatim — shared `super::TAKE_FIRST_TIMEOUT`
in both seams, cure messages exact, sweep keeps `pnl_single conid {conid}:` attribution,
`Some(Ok)`/`Some(Err)` arms byte-identical, module docs updated to cite ADR 0012.

Adversarial subagent attempted to REFUTE 6 claims; ALL CONFIRMED with ibapi-3.1.0 + branch
citations (highlights):

1. Healthy-path byte-identity: `next_routed()` (`recv().ok()`) and `next_timeout_routed()`
   (`recv_timeout().ok()`) deliver the identical `RoutedItem`; decode/notice-filter/stream_ended
   machinery shared (sync.rs:145-195, 242-244, 279-281; transport/mod.rs:107-121).
2. Cancel/Drop unchanged: timeout iterators are stateless `&Subscription` borrows with no Drop
   impl; the only cancel is `Subscription::drop`, at the same scope-exit points as pre-PR.
3. Timeout semantics: ~10s on silent-open; INSTANT `None` on ended stream (no 10s wait);
   per-notice window restart (N notices ⇒ up to (N+1)×10s) — matches ADR 0012's stated caveat.
4. No unbounded take-first read remains: zero `next_data()` call sites on the branch; brief
   consumes the two seams only.
5. Envelope safe: exhaustive matches, additive-only; no other `ErrorKind` dispatch in src/.
6. Frozen spec honest: table matches impl exactly; test imports could not compile on main
   (red-before proven by grep).

## Live acceptance (PRD criterion 8) — operator gateway, live :4001, 2026-07-03 PM

PR-head binary from the detached worktree. Account ids/balances redacted — public repo.

- `omi --live health` ✓ connected, server v221.
- **`omi --live pnl` ✓ PASS — 2.9s, exit 0**, all 3 PnL keys numeric (first reqPnL of the
  gateway session): the happy path is NOT slowed by the bound (criterion 8).
- **Timeout path proven LIVE 4×** (bonus proof per criterion 8, not a required trigger): the
  known wedge reproduced — with NO process kill involved — and every starved read exited
  **bounded at 10.2–10.3s, exit 6, exact envelope**
  `{"error":{"code":"timeout","context":"brief/pnl"|"pnl","message":"no PnL reading within 10s — gateway PnL channel may be wedged; restart the gateway"}}`.
  Pre-PR behavior on the same wedge (same morning, brief-command acceptance): infinite hang +
  orphaned subscriptions requiring SIGKILL.
- **brief healthy-path**: no clean PASS obtained THIS session — the wedge starved every reqPnL
  after the session's first (see finding below; `pnl` had taken the first slot). Criterion-8
  coverage for brief's happy path rests on: (a) the seam `pnl_with_client` — the ONLY changed
  code brief executes — passed live at 2.9s; (b) brief.rs is untouched and the seams'
  success arms are byte-identical (adversarial claim 1/4); (c) brief passed live acceptance
   this morning (3.3s) on the pre-PR build of the identical happy path.
  **Optional strengthening run (operator, next gateway restart): run `omi --live brief` FIRST
  (before any pnl/brief), expect ~3s exit 0.** May happen before or after merge — it gates
  nothing in this diff, only strengthens the record.

## Side-finding (gateway, NOT this diff) — wedge characterization STRENGTHENED

Two sessions / 8 data points now show: **on this gateway build (2026-06-25 stable, M1 install),
only the FIRST reqPnL subscription per gateway session delivers; every later one starves** —
kills are NOT required to trigger it (all clients this session exited cleanly, cancel sent on
Drop). AM session: brief#1 PASS → pnl, brief#2 starved. PM session: pnl#1 PASS 2.9s →
brief×3 + pnl starved (all bounded post-PR). Suspected mechanism: the gateway pushes PnL only
on change/initial-snapshot to the first subscriber; a flat account never changes intraday, so
later subscribers wait forever (alternatively: cancel leaves the channel unreleased).
Operational guidance recorded: **run `omi --live brief` first after gateway login**; a restart
buys exactly one reqPnL slot. Also observed once (07:20 UTC): a 60s+ hang in the CONNECT phase
during an unhealthy window — connect timeouts are explicitly non-scope here (connect-retry
feature owns that posture); single data point, recorded for evidence only.

## Disposition

- APPROVE stands on: freeze gate EMPTY + full suite 71/71 + clippy clean + verbatim-arch
  conformance + 6/6 adversarial confirmations + live: pnl happy-path PASS and the timeout
  path's 4× live proof (the feature's entire purpose, demonstrated against the real hazard).
- Merge remains gated on explicit operator confirmation (CONTRACT: only pipeline-review merges,
  human-confirmed). The optional brief first-slot run is recorded above and does not block.
- Recommended follow-ups (operator decides, NOT this PR): (a) the standing reqPnLSingle
  first-trading-day observation (pnl-by-position D6); (b) consider reporting/investigating the
  first-slot-only reqPnL behavior of gateway build 2026-06-25 (upgrade path or IB report).
