# review-01 — preview-readonly PR #24

verdict: PASS
reviewer: codex
writer: π (GLM-5.2)
pr: https://github.com/jackypanster/oh-my-ib/pull/24
head: df43bd1d9c0907de0a3fdd8a33cb95c3f3234a08
base: origin/main
review-depth: deep (money-adjacent order preview path)
scope: on-target
findings: none
merge: DO NOT MERGE until explicit operator confirmation

## Freeze Gate

PASS. Command run:

```sh
git diff 5dce9574e3ce938d79cce3be8bd59daf22bdb39e origin/feat/preview-readonly -- tests/order_preview_command.rs
```

Output was empty. The PR branch did not touch the frozen spec after `spec-rev`.

## Containment

PASS. The safety property is structural: preview calls no `place_order`.

```sh
git grep -n -E "\.place_order\(|\.cancel_order\(" origin/feat/preview-readonly -- src/ib/trade.rs
```

Executable write calls appear only at:

- `src/ib/trade.rs:319` — `.cancel_order(...)` in `cancel`.
- `src/ib/trade.rs:371` — `.place_order(...)` in `place_with_client`, the real order choke point.

`preview_with_client` is gone from `src/ib/trade.rs`; the new preview path is `preview_stk_option` at `src/ib/trade.rs:415`, which uses `client.contract_details` at `src/ib/trade.rs:421`.

## Implementation Checks

PASS. Impl reviewed against `arch.md` Component changes:

- `src/ib/trade.rs:79` changes `shape_preview` to the new read-only signature:
  `Value contract, &Order, multiplier, ccy`.
- `src/ib/trade.rs:85` computes notional from `limit_price.map(|l| qty * abs(limit) * multiplier)`, producing JSON null for MKT.
- `src/ib/trade.rs:88` emits the read-only envelope with `transmits:false`, `notional`, `notional_currency`, and no `what_if`/`margin`/`commission`/`status`.
- `src/ib/trade.rs:421` uses `contract_details` for STK/single-leg option preview and calls `shape_preview`; no order submission.
- `src/ib/trade.rs:475` branches preview before `require_live_write_gate` in `place_core`.
- `src/ib/trade.rs:766` and `src/ib/trade.rs:902` make the combo/close write gate conditional on `!cfg.preview`; real arms still call through `place_with_client`.
- `src/ib/trade.rs:820` and `src/ib/trade.rs:1098` shape combo/close previews from already-resolved data and call no write API.

Real-path checks:

- `place_with_client` body hash matches `origin/main`: `94a281fa7ef9af69ffddd39b1a0a5c25f29ec58a6ee08692de1dac1cc84277c7`.
- `require_live_write_gate` body hash matches `origin/main`: `55567f758a3503937c20e7426146330695dc889b0e296c9a4d9346d013dad17a`.
- Frozen tests keep real buy/sell `--live` without `OMI_ALLOW_LIVE` at `config`.

## Verification

All commands were run in a detached worktree at PR head `df43bd1d9c0907de0a3fdd8a33cb95c3f3234a08`:

```sh
cargo build
cargo clippy --all-targets -- -D warnings
cargo test
```

Results: all passed. `cargo test` included `tests/order_preview_command.rs` with 12/12 passing.

## Non-Blocking Live Acceptance

Not frozen, not a review blocker per the card and handoff:

- read-shaped live gate: `--live --preview` without `OMI_ALLOW_LIVE` reaches connect, not `config`.
- cc live-acceptance: `omi --live buy AAPL 1 --limit 1 --preview` returns the read-only envelope and `omi --live orders` remains empty.

These remain operator/cc live checks; this review confirms the structural no-transmit property by code containment.
