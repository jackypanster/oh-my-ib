---
feature: grid-tick
card: "01"
pr: 30
head: c2147f9efc900c599e8a5843d3fecbc5a1f54749
verdict: REJECT
reviewer: codex
date: 2026-07-07
---

# Review 01 -- grid-tick card 01

## Verdict

REJECT.

## Blocking Finding

- `src/ib/grid.rs:37` -- `grid_tick` resolves one account for positions and future placements, but reads
  open orders with `cfg.account.as_deref()`. When the operator omits `--account`, `cfg.account` is `None`,
  and `open_orders_with_client` deliberately does not filter rows in that case (`src/ib/orders.rs:30-33`).
  On a multi-account gateway, this can include another account's open order for a configured symbol. The
  planner then treats that order as owned grid state and may emit `Cancel`, while new places are stamped to
  the resolved first account. Trigger: two managed accounts, no `--account`, configured symbol `AAPL`, and
  an `AAPL` open order on the non-resolved account. Existing guards do not prevent this: `cancel_with_client`
  cancels by order id, and the row is only filtered when an explicit account string is passed.

Fix: after `let account = super::resolve_account(&client, cfg)?;`, call
`open_orders_with_client(&client, Some(account.0.as_str()), "grid-tick")` (or equivalent) so the read-side
reconcile surface matches the same resolved account used for `read_account_positions` and `place_with_client`.

## Gates

- Freeze gate: PASS. `git diff 4b83d2a origin/feat/grid-tick -- tests/grid_tick.rs` was empty.
- PR surface: PR #30 is open/mergeable at `c2147f9efc900c599e8a5843d3fecbc5a1f54749`, 8 files, +507/-12.
- Four prior write tests are byte-identical in the PR diff:
  `tests/stk_orders_command.rs`, `tests/order_preview_command.rs`,
  `tests/write_path_semantics_doc.rs`, `tests/live_write_guardrail.rs`.
- Full verify on PR head: PASS.
  - `cargo build`
  - `cargo test`
  - `cargo clippy --all-targets -- -D warnings`
- CLI smoke: PASS. `./target/debug/omi grid-tick --help` exposes `--config` and `--dry-run`;
  `./target/debug/omi --format json --live grid-tick --config /tmp/omi-grid-tick-review.toml --dry-run`
  exits 5 with `code=config`, `context=grid-tick` before connect/write.
- CI status checked with `gh pr checks 30`: CodeRabbit pass.

## Semantic Review Notes

- ADR 0017 containment holds by grep: actual raw `place_order` / `cancel_order` calls remain only in
  `src/ib/trade.rs`; `src/ib/grid.rs` composes `build_stk_order`, `place_with_client`, and
  `cancel_with_client`.
- `plan_grid_tick` matches ADR 0033 D-PLANNER for the frozen cases: sell rung, cash-floor buy suppression,
  strict `qty + lot <= max_shares`, flat cancels lingering orders, idempotent keep tolerance, and global
  cancels-before-places.
- `place_with_client` reuse preserves ADR 0024 account stamping; grid passes the resolved account to the
  choke point.
- `cancel` public behavior is preserved by extraction/delegation; the existing public wrapper still gates,
  connects, and then calls the extracted bounded-ack body.
- Operator paper acceptance remains deferred: paper `:4002` lifecycle requires a live paper gateway and
  should be run after the blocking account-scope fix lands.
