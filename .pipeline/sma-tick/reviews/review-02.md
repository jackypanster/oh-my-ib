---
feature: sma-tick
card: "01"
pr: 32
head: daa6af28ea568679a6770946b89bc074e32e3b95
verdict: ACCEPT
reviewer: codex
date: 2026-07-07
---

# Review 02 -- sma-tick card 01

## Verdict

ACCEPT.

## Findings

None.

## Re-Review Focus

- Review-01 finding 1 is fixed. `sma_tick_cmd` now calls `current_position_qty(cfg, &symbol)?`;
  `current_position_qty` returns `Result<f64, AppError>` and uses `super::positions(cfg)?`. Position
  read failures propagate before planning/placement. Only "symbol absent from a successful positions
  payload" maps to `0.0`.
- Review-01 finding 2 is fixed. `sma_tick_cmd` validates `!args.lot.is_finite() || args.lot <= 0.0`
  before `connect(cfg)`, returning `AppError::config(..., "sma-tick")`. `build_stk_order` is reached
  only with a finite positive lot-derived quantity.
- Previously accepted surfaces remain intact: pure `plan_sma_tick`, marketable LMT prices, `signal_for`
  reuse, paper-only guard, ADR 0017 containment, and QQQM default.

## Gates

- Freeze gate: PASS. `git diff 22b1a9e origin/feat/sma-tick -- tests/sma_tick.rs` is empty.
- Full verify on PR head `daa6af28ea568679a6770946b89bc074e32e3b95` in a detached worktree: PASS.
  - `cargo build`
  - `cargo test`
  - `cargo clippy --all-targets -- -D warnings`
- Existing frozen/regression suites are byte-identical in the PR diff: `tests/sma_signal.rs`,
  `tests/grid_tick.rs`, `tests/stk_orders_command.rs`, `tests/order_preview_command.rs`,
  `tests/write_path_semantics_doc.rs`, and `tests/live_write_guardrail.rs`.
- Containment grep: PASS. `src/ib/sma_tick.rs` has no code hit for raw `place_order` or `cancel_order`;
  the only hit is the module doc comment.
- Paper-only smoke: PASS. `./target/debug/omi --live sma-tick QQQM` exits 5 with `code=config`,
  `context=sma-tick`, before connect.
- Lot validation smokes: PASS. `--lot=0`, `--lot=inf`, and `--lot=-10` all exit 5 with `code=config`,
  `context=sma-tick`, before connect.
- CLI default smoke: PASS. `omi sma-tick --help` reports `[SYMBOL]` default `QQQM`.
- CI status checked with `gh pr checks 32`: CodeRabbit pass.

## Merge Note

GitHub still reports PR #32 as `CONFLICTING`, but local `git merge-tree` shows the conflict is limited
to `.pipeline/sma-tick/tasks/01.md` metadata (main has the review status; the PR branch carries the QQQM
task-doc edit plus stale `status: todo`). The `src/**` merge is clean. Per the review request, this
metadata skew is not treated as a source-code rejection; the human merge step should reconcile metadata
while preserving trunk review state.

## Deferred

- Operator paper acceptance remains deferred until after human-confirmed merge and a live paper gateway
  run.
