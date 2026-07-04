# Review 01 — option-combo

Verdict: REJECT

## Context note

The dispatch named `.pipeline/option-combo/PRD.md`, `arch.md`, and `docs/adr/0021`, but those files are not present at trunk `9290b24`. The feature journal seq=4 explicitly says the pipeline compressed this feature into the card and frozen test: "No arch.md/ADR/CONTEXT/PRD committed for this feature; the card §Scope + frozen test ARE the contract." This review used `tasks/01.md`, `tests/option_combo_command.rs`, and the journal tail as the binding context.

## Gate evidence

- Freeze gate: PASS. `git diff c0e72a3 origin/feat/option-combo -- tests/option_combo_command.rs` produced empty output.
- Freeze gate, whole `tests/`: PASS. `git diff origin/main origin/feat/option-combo -- tests/` produced empty output, so combo, stk, option-orders, docs, and other frozen specs are untouched.
- Full suite on detached PR head `/tmp/omi-rev-oc-branch` at `6d470a1`: PASS.
  - `cargo build`: pass.
  - `cargo test`: pass, 181 tests across 23 suites; `tests/option_combo_command.rs` 21/21, `tests/option_orders_command.rs` 21/21, `tests/stk_orders_command.rs` 16/16.
  - `cargo clippy --all-targets -- -D warnings`: pass.
- PR surface: `AGENTS.md`, `CLAUDE.md`, `src/cli.rs`, `src/ib/mod.rs`, `src/ib/trade.rs`, `src/main.rs`.
- `git diff --check origin/main...origin/feat/option-combo`: pass.

## Blocking finding

1. `src/ib/trade.rs:552` / `src/ib/trade.rs:597` — `option_combo` opens one gateway connection for conid resolution, then calls `place_core`, which opens a second connection with the same config/client id before allocating and placing.

   Trigger: any valid combo order against a reachable gateway. The function performs:

   - `require_live_write_gate(cfg)?` at `src/ib/trade.rs:549`.
   - `let client = super::connect(cfg)?` at `src/ib/trade.rs:552`.
   - `client.contract_details(...)` for each leg at `src/ib/trade.rs:573-575`.
   - `place_core(cfg, ...)` at `src/ib/trade.rs:597`.

   `place_core` then calls `super::connect(cfg)?` again at `src/ib/trade.rs:221`, and only afterward allocates `client.next_order_id()` at `src/ib/trade.rs:227` and calls `place_order` at `src/ib/trade.rs:230-231`.

   There is no `drop(client)` or client-passing path before `place_core`, so the contract-details client remains in scope while the placement core attempts a second connection using the same `cfg.client_id`. This is a live write-path hazard: `src/ib/mod.rs:56-58` already documents transient reconnect failures when the gateway has not released the prior same-client-id session, and here the prior session is still active.

   Expected: one connected placement sequence after validation/gate, with per-leg `contract_details` resolution strictly before `place_order` and no second same-client-id connection. A safe shape is to refactor the shared placement core so combo can reuse the already-connected `Client` for `next_order_id()`, `place_order`, and the bounded first-ack loop while preserving byte-identical behavior for STK and single-leg option verbs.

   This is not caught by dead-port tests because a dead port fails at the first `connect`; it only appears with a reachable gateway, exactly the layer this semantic review must carry.

## Semantic checks passed by reading

- `parse_combo_leg` uses `split_whitespace`, so leading/trailing whitespace, tabs, and repeated spaces parse as token separators.
- `parse_combo_leg` rejects bad action, non-integer/overflow/negative/zero ratio, bad expiry, non-finite or non-positive strike, and bad right; errors are wrapped by the gateway as `leg N: ...`.
- `build_combo_order` uses `Contract::spread()`, adds combo legs in input order via `.add_leg(...).ratio(...).done()`, applies exchange/currency, calls `.build()`, back-fills `contract.symbol`, and builds an LMT/DAY order with `limit_price: Some(limit)`.
- Net combo limit is sign-free by design: negative, zero, and positive finite limits are accepted; non-finite limits are rejected upstream.
- Per-leg conid resolution is attempted in input order; `contract_details` uses the first row; failures include `leg N`; `place_order` is after the resolution loop.
- Existing STK and single-leg option suites stayed green, and the PR diff does not change `place_core` or existing placement call sites except for imports and the new combo code.
- Write-call containment: `rg -n "place_order|submit_order|encode_place_order|cancel_order" -S src` hits only `src/ib/trade.rs`. `contract_details` is a read call and is only used for conid resolution.
- No combo MKT arm exists; combo order type is `"LMT"` and TIF is `TimeInForce::Day`. No placement retry logic was introduced.
- CLI wiring uses singular `--leg` with `Vec<String>` and `allow_hyphen_values` on `--limit`.
- Docs two-text rule matches the card/journal: AGENTS full form, CLAUDE short form; `wc -c CLAUDE.md` is `861`, under the frozen `< 900` budget.
- `git diff --exit-code origin/main...origin/feat/option-combo -- Cargo.toml Cargo.lock` is empty; no new dependencies.
- Read modules are untouched: `git diff --exit-code origin/main...origin/feat/option-combo -- src/ib/quote.rs src/ib/option_chain.rs src/ib/option_quote.rs src/ib/account.rs src/ib/portfolio.rs src/ib/positions.rs src/ib/orders.rs src/ib/completed_orders.rs src/ib/search.rs src/ib/pnl.rs src/ib/pnl_by_position.rs src/ib/executions.rs` is empty.
- Secret/account scan found no new credential or real account material in the PR diff; the only account-like value is an existing fake test fixture.

## Behavioral probes

All probes used `/tmp/omi-rev-oc-branch/target/debug/omi` with `env -u OMI_ALLOW_LIVE` and only dead-port or pre-gate live cases.

- Whitespace: tab/multiple-space/leading/trailing leg strings reached `code="connection"` on `127.0.0.1:65000`, proving they parse as valid.
- Parser adversarial: ratio `99999999999`, ratio `-1`, and Unicode ratio `１` returned `code="usage"` and named `leg 1`.
- Validation ordering: mixed underlyings with `--live` returned `code="usage"`, not `config`.
- Non-finite net limit: `--limit inf` and `--limit NaN` returned `code="usage"`.
- Sign-free finite net limit: `--limit -0.50`, `--limit 0`, and `--limit -0` reached `code="connection"` on the dead paper port.
- Live gate: valid `--live` with no `OMI_ALLOW_LIVE` returned `code="config"`; hand-set `--port 4001` without `--live` returned `code="config"`.
