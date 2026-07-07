---
feature: sma-tick
card: "01"
pr: 32
head: 9a71bfb737f8871f852113f0893e590552a163a3
verdict: REJECT
reviewer: codex
date: 2026-07-07
---

# Review 01 -- sma-tick card 01

## Verdict

REJECT.

## Blocking Findings

- `src/ib/sma_tick.rs:82`, `src/ib/sma_tick.rs:154-168` -- `sma_tick_cmd` treats any failure while reading
  current positions as `current_qty = 0.0`. Trigger: paper gateway successfully connects, resolves the
  account, and returns the SMA signal, but the subsequent `positions(cfg)` call fails or its
  `account_updates` stream errors. Existing guards do not prevent an unsafe order: the planner then acts
  on a fabricated flat position. In a HOLD state it can buy `lot` even when the account already holds the
  target position; in an EXIT state it can no-op instead of closing an unknown held position. The card
  explicitly specified `super::positions(cfg)?`, not best-effort fallback. For a write command, unknown
  position state must fail closed before planning or placement.

  Fix: make the position read return `Result<f64, AppError>` and propagate errors with `?`, or preferably
  read positions through the already-resolved account/client (same shape as grid-tick's
  `read_account_positions`) so the signal, position, and placement surfaces share one account authority.
  Only "symbol absent from a successful positions payload" should mean 0.0.

- `src/cli.rs:311`, `src/ib/sma_tick.rs:83`, `src/ib/sma_tick.rs:117-121` -- `--lot` accepts invalid
  values for a mutating command. Trigger: with a HOLD signal and flat account, `--lot=-10` makes
  `target=-10`, `delta=-10`, then `plan_sma_tick` returns `Sell { qty: 10 }`; `--lot=inf` can produce an
  infinite order quantity. Existing trade validation does not catch this because `sma_tick_cmd` bypasses
  the public `buy`/`sell` validators and calls `build_stk_order` directly; `build_stk_order` is a pure
  builder and accepts the quantity verbatim.

  Fix: validate `args.lot.is_finite() && args.lot > 0.0` in `sma_tick_cmd` before any gateway work,
  returning a structured usage/config error with context `sma-tick`. Add implementation-owned coverage for
  negative, zero, and non-finite lot values without touching `tests/sma_tick.rs`.

## Gates

- Freeze gate: PASS. `git diff 22b1a9e origin/feat/sma-tick -- tests/sma_tick.rs` was empty.
- PR surface: PR #32 head is `9a71bfb737f8871f852113f0893e590552a163a3`. The src review surface is 5
  files, +217/-16: `src/cli.rs`, `src/ib/mod.rs`, `src/ib/signal.rs`, `src/ib/sma_tick.rs`, `src/main.rs`.
- Expected metadata noise: PR #32 also contains `.pipeline/sma-tick/tasks/01.md` from the operator's
  QQQM metadata sync commit. Per the review request, this was not used as a rejection basis. GitHub reports
  the PR as `CONFLICTING`, apparently because of that `.pipeline` skew; src review findings above are
  independent of the metadata noise.
- Prior/frozen suites are byte-identical in the src review diff: `tests/sma_signal.rs`, `tests/grid_tick.rs`,
  `tests/stk_orders_command.rs`, `tests/order_preview_command.rs`, `tests/write_path_semantics_doc.rs`,
  and `tests/live_write_guardrail.rs` were unchanged.
- Full verify on PR head in an isolated worktree: PASS.
  - `cargo build`
  - `cargo test`
  - `cargo clippy --all-targets -- -D warnings`
- Containment grep: PASS. `src/ib/sma_tick.rs` has no code hit for raw `place_order` or `cancel_order`;
  the only grep hit is the module doc comment.
- Paper-only guard smoke: PASS. `./target/debug/omi --live sma-tick QQQM` exits 5 with
  `code=config`, `context=sma-tick`, before connect.
- Default symbol: PASS. CLI help and `sma_tick_cmd` both show/use `QQQM`.
- CI status checked with `gh pr checks 32`: CodeRabbit pass.

## Semantic Review Notes

- `plan_sma_tick` matches ADR 0035 for the frozen positive-lot cases: HOLD targets `lot`, EXIT targets
  zero, INSUFFICIENT no-ops, and epsilon comparisons avoid float equality.
- Marketable LMT pricing matches ADR 0035: Buy at `round2(latest_close * 1.02)` and Sell at
  `round2(latest_close * 0.98)`.
- `signal_for` extraction keeps `sma_signal_cmd` behavior structurally equivalent and `tests/sma_signal.rs`
  remains green.
- Operator paper acceptance remains deferred until the blocking fail-closed issues above are fixed.
