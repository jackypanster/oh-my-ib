---
feature: sma-signal
card: "01"
pr: 31
head: 5ea2a8b7de3665ffd20c816e63c41259aec764da
verdict: ACCEPT
reviewer: codex
date: 2026-07-07
---

# Review 02 -- sma-signal card 01

## Verdict

ACCEPT.

## Findings

None.

## Re-Review Focus

- Review-01 blocking finding is fixed. `src/ib/signal.rs` now validates `args.sma < 1` at the start of
  `sma_signal_cmd`, before `held_symbols(cfg)` and before `super::connect(cfg)`, returning
  `AppError::config("--sma must be >= 1, got 0", "sma-signal")`.
- The pure seam is now total for `n == 0`: `sma_signal` returns `SignalState::Insufficient` through the
  widened `if n == 0 || bars.len() < n` guard, so it cannot reach `sma_at` with `start > i`.
- The positive-window Faber rule is otherwise unchanged from review-01: last completed month selection,
  SMA-as-of-month-end, `Insufficient` history handling, and latest drift still match ADR 0034.

## Gates

- Freeze gate: PASS. `git diff 58f31d4 origin/feat/sma-signal -- tests/sma_signal.rs` was empty.
- PR surface: PR #31 is open/mergeable at `5ea2a8b7de3665ffd20c816e63c41259aec764da`, 6 implementation
  files, +255/-0.
- Existing suites are byte-identical in the PR diff: `git diff origin/main...origin/feat/sma-signal -- tests`
  was empty.
- Full verify on PR head in an isolated worktree: PASS.
  - `cargo build`
  - `cargo test`
  - `cargo clippy --all-targets -- -D warnings`
- CLI zero-window smoke: PASS. `./target/debug/omi sma-signal --sma 0 NVDA` exited 5 with
  `{"error":{"code":"config","context":"sma-signal","message":"--sma must be >= 1, got 0"}}`, proving the
  failure is structured and pre-connect rather than a connection error or panic.
- Read-only posture grep: PASS. `signal.rs` has no code hit for `place_order`, `cancel_order`,
  `require_live_write_gate`, or `OMI_ALLOW_LIVE`; the only hit is the intentional module doc comment on
  line 11.
- CI status checked with `gh pr checks 31`: CodeRabbit pass.

## Deferred

- Operator paper acceptance remains deferred to the paper gateway workflow: `omi sma-signal NVDA MU QQQ`
  returns HOLD/EXIT signals and `omi sma-signal` signals current positions.
