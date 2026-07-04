# Review 02 — option-combo

Verdict: PASS

## Scope

Incremental re-review after review-01 rejected the combo write path for a double-connect hazard.
Round-1 evidence stands for the unchanged parser, validation, order-shaping, docs, dependency, and
read-module surface. This review focused on branch tip `5def158` over `6d470a1`.

## Gate evidence

- Freeze gate: PASS. `git diff c0e72a3 origin/feat/option-combo -- tests/option_combo_command.rs`
  produced empty output.
- Freeze gate, whole `tests/`: PASS. `git diff origin/main origin/feat/option-combo -- tests/`
  produced empty output, so combo, STK, option-orders, docs, and other frozen specs are untouched.
- Round-2 product diff scope: PASS. `git diff 6d470a1..5def158 --name-only` shows only
  `src/ib/trade.rs`; `git diff --exit-code 6d470a1..5def158 -- AGENTS.md CLAUDE.md src/cli.rs
  src/ib/mod.rs src/main.rs tests Cargo.toml Cargo.lock` is empty.
- Full regression on detached PR head `/tmp/omi-rev-oc2-branch` at `5def158`: PASS.
  - `cargo build`: pass.
  - `cargo test`: pass, 181 tests across 23 suites; `tests/option_combo_command.rs` 21/21,
    `tests/option_orders_command.rs` 21/21, `tests/stk_orders_command.rs` 16/16.
  - `cargo clippy --all-targets -- -D warnings`: pass.
- `git diff --check origin/main...origin/feat/option-combo`: PASS.

## Semantic review

- `src/ib/trade.rs:210` adds private `place_with_client(client: &Client, ...)`, containing the old
  placement body from `client.next_order_id()` through `client.place_order(...)` and the bounded
  first-ack loop under `TAKE_FIRST_TIMEOUT`.
- `src/ib/trade.rs:267` keeps private `place_core(...)` as a thin wrapper:
  `require_live_write_gate(cfg)?` -> `super::connect(cfg)?` -> `place_with_client(&client, ...)`.
  Existing STK and single-leg option call sites still call `place_core` at `src/ib/trade.rs:301`
  and `src/ib/trade.rs:369`.
- `src/ib/trade.rs:562` connects once in `option_combo`; the same `client` resolves every leg with
  `contract_details` at `src/ib/trade.rs:583`, then places via `place_with_client(&client, ...)` at
  `src/ib/trade.rs:607`. The review-01 double-connect finding is dead.
- `super::connect` attribution in `src/ib/trade.rs` is exactly three call sites:
  `cancel` at line 178, the `place_core` wrapper at line 275, and `option_combo` at line 562.
  There is no hidden second connect on the combo placement path.
- Write containment still holds: `rg -n "place_order|submit_order|encode_place_order|cancel_order" -S src`
  hits only `src/ib/trade.rs`.

## Handoff

Do not merge from this review. PR #18 can proceed to operator paper acceptance and explicit human
confirmation. The final merge step should re-run the normal gates before squash-merging.
