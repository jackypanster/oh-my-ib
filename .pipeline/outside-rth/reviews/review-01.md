---
feature: outside-rth
card: "01"
pr: 29
head: b4728736607f743b2cb6bb671e1eeca3fb79e375
verdict: ACCEPT
reviewer: codex
date: 2026-07-07
---

# Review 01 -- outside-rth card 01

## Verdict

ACCEPT.

## Findings

None.

## Gates

- Freeze gate: PASS. `git diff bd9a1e8..origin/feat/outside-rth -- tests/outside_rth.rs` was empty.
- PR surface: PASS. GitHub PR #29 / merge-base diff is exactly 3 files, +21/-2:
  `src/cli.rs`, `src/ib/trade.rs`, `src/ib/mod.rs`.
- Full verify on PR head `b4728736607f743b2cb6bb671e1eeca3fb79e375`: PASS.
  - `cargo build`
  - `cargo test`
  - `cargo clippy --all-targets -- -D warnings`

## Semantic Review

- `build_stk_order` remains 4-arg; signature and body are unchanged in the PR diff.
- `shape_order_ack` remains the existing 6-key transmit ack and is not extended.
- `shape_preview` keeps the same 8 top-level keys; only nested `order.outside_rth` is added.
- `apply_outside_rth` rejects only `MKT + outside_rth=true`, returns a message containing `limit`, and
  does not mutate the flag before returning that error. All other cases set `order.outside_rth`
  verbatim.
- `place()` applies the flag after `build_stk_order` and before `place_core`, maps errors through
  `AppError::config`, and therefore refuses offline/pre-connect with exit bucket 5.
- Scope stayed STK-only: no option/combo/close argument or order path was changed.
