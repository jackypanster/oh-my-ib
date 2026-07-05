# review-01 — order-preview PR #23

verdict: PASS
reviewer: codex
writer: π (GLM-5.2)
pr: https://github.com/jackypanster/oh-my-ib/pull/23
head: 6c6ba37cd3986e1b5cf3c20fc6330cabf13938a8
base: origin/main
review-depth: deep (mutating order path)
scope: on-target
findings: none
merge: DO NOT MERGE until explicit operator confirmation

## Freeze Gate

PASS. Command run:

```sh
git diff 0914c912468be16d3acbc97069c935b87ca302b8 origin/feat/order-preview -- tests/order_preview_command.rs
```

Output was empty. The seq=5 re-freeze supersedes seq=4; no doc-comment adjudication remains.

## Implementation Checks

PASS. Impl-paths reviewed against `arch.md` Data flow and The two new seams:

- `src/cli.rs:34` adds global `--preview`.
- `src/config.rs:44` adds CLI-only `Config.preview`; `src/config.rs:56` defaults false; `src/config.rs:134` merges `g.preview`.
- `src/ib/mod.rs:45` re-exports `shape_preview`.
- `src/ib/trade.rs:77` implements pure `shape_preview(&Contract, &Order, &OrderState) -> Value`; it emits the 9 top-level keys and uses `json!`, so `Option<f64>::None` serializes as JSON null while keys stay present.
- `src/ib/trade.rs:430` clones and stamps the resolved account; `src/ib/trade.rs:432` sets `order.what_if = true` before `place_order`; `src/ib/trade.rs:443` returns `shape_preview(contract, &order, &od.order_state)` on the first `OpenOrder`.
- `src/ib/trade.rs:478` runs `require_live_write_gate(cfg)?` before the stock/single-leg preview branch at `src/ib/trade.rs:481`.
- `src/ib/trade.rs:768` runs the combo gate before the preview branch at `src/ib/trade.rs:820`.
- `src/ib/trade.rs:878` runs the close gate before the preview branch at `src/ib/trade.rs:1072`.

Gate identity:

- `require_live_write_gate` body hash is identical between `origin/main` and `origin/feat/order-preview`: `55567f758a3503937c20e7426146330695dc889b0e296c9a4d9346d013dad17a`.
- `place_with_client` body hash is identical between `origin/main` and `origin/feat/order-preview`: `94a281fa7ef9af69ffddd39b1a0a5c25f29ec58a6ee08692de1dac1cc84277c7`.
- Preview is not read-shaped or ungated; it reaches the gateway only after the same write gate as a real order.

Write containment:

```sh
git grep -n -E "place_order|cancel_order" origin/feat/order-preview -- src
```

Executable gateway calls appear only in `src/ib/trade.rs`: `cancel_order` at line 323, real `place_order` at line 375, preview `place_order` at line 437.

## Verification

All commands were run in a detached worktree at PR head `6c6ba37cd3986e1b5cf3c20fc6330cabf13938a8`:

```sh
cargo build
cargo clippy --all-targets -- -D warnings
cargo test
```

Results: all passed. `cargo test` included `tests/order_preview_command.rs` with 11/11 passing.

## Non-Blocking Live Acceptance

Not frozen, not a review blocker per `CONTEXT.md` R1/R2: operator must still confirm on Tiger live that `Order.what_if=true` does not transmit and that margin/commission behavior is acceptable:

```sh
omi --live buy <sym> 1 --limit <far-below-market> --preview
omi --live orders
```

Expected: preview envelope returns, then no resting order appears for the previewed symbol/order.
