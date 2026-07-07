---
feature: sma-signal
card: "01"
pr: 31
head: b2a040b9f94a213b157150d76241e9007a811e45
verdict: REJECT
reviewer: codex
date: 2026-07-07
---

# Review 01 -- sma-signal card 01

## Verdict

REJECT.

## Blocking Finding

- `src/cli.rs:297`, `src/ib/signal.rs:197`, `src/ib/signal.rs:149-152` -- `--sma 0` is accepted by
  clap and reaches `sma_signal(&bars, 0)`, which can panic after the gateway returns any non-empty
  historical bar set. Trigger: `omi sma-signal --sma 0 NVDA` against a paper/live gateway that returns at
  least one bar. Existing guards do not prevent it: `bars.len() < n` and `as_of_idx + 1 < n` are both
  false when `n == 0`; `sma_at` then computes `start = i + 1 - n`, so `start == i + 1`, and slices
  `bars[start..=i]` with `start > i`. That violates the repo CLI contract that errors return a structured
  JSON envelope instead of a panic.

Fix: make the SMA window positive at the public boundary, preferably rejecting `args.sma == 0` in
`sma_signal_cmd` with `AppError::config` or a clap range parser before any gateway work. Also make the
pure seam total for `n == 0` (for example return `Insufficient`) so direct library callers cannot panic.
Keep `tests/sma_signal.rs` untouched; add implementation-owned coverage if needed.

## Gates

- Freeze gate: PASS. `git diff 58f31d4 origin/feat/sma-signal -- tests/sma_signal.rs` was empty.
- PR surface: PR #31 is open/mergeable at `b2a040b9f94a213b157150d76241e9007a811e45`, 6 files, +245/-0.
- Existing suites are byte-identical in the PR diff: `git diff origin/main...origin/feat/sma-signal -- tests`
  was empty.
- Full verify on PR head in an isolated worktree: PASS.
  - `cargo build`
  - `cargo test`
  - `cargo clippy --all-targets -- -D warnings`
- Read-only posture grep: PASS. `signal.rs` has no code hit for `place_order`, `cancel_order`,
  `require_live_write_gate`, or `OMI_ALLOW_LIVE`; the only hit is the intentional module doc comment on
  line 11.
- CLI smoke: PASS. `target/debug/omi sma-signal --help` exposes the command and `--sma`.
- CI status checked with `gh pr checks 31`: CodeRabbit pass.

## Semantic Review Notes

- The normal `sma_signal` D-RULE matches ADR 0034 for positive `n`: it excludes the in-progress final
  month, falls back to the final bar for a single-month series, computes SMA at `as_of_idx`, and returns
  `Insufficient` when the chosen month-end lacks enough history.
- Gateway read path is read-only and uses the intended surfaces: no-args resolves held symbols from
  `positions(cfg)`, data fetch uses `historical_data(..., BarSize::Day).what_to_show(Trades).duration(2y)`,
  `ym_of` strips both `Date` and `DateTime` variants to `(i32, u32)`, and output is the ADR JSON envelope.
- The direct `time = "0.3"` dependency is justified by `ym_of` needing the `time` calendar accessors.
- Operator paper acceptance remains deferred: `omi sma-signal NVDA MU QQQ` and no-args current-position
  signaling require a live paper gateway.
