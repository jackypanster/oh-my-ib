# Review 01 — option-orders

Verdict: REJECT

## Gate evidence

- Freeze gate: PASS. `git diff 63f3232 origin/feat/option-orders -- tests/option_orders_command.rs` produced empty output.
- Freeze gate, whole `tests/`: PASS. `git diff origin/main origin/feat/option-orders -- tests/` produced empty output, so the option-order spec and existing stk spec are untouched.
- Full suite on detached PR head `/tmp/omi-rev-oo-branch` at `2589341a817c`: PASS.
  - `cargo build`: pass.
  - `cargo test`: pass, 160 tests across 22 suites; `tests/option_orders_command.rs` 21/21 and `tests/stk_orders_command.rs` 16/16.
  - `cargo clippy --all-targets -- -D warnings`: pass.
- PR surface: `AGENTS.md`, `CLAUDE.md`, `src/cli.rs`, `src/ib/mod.rs`, `src/ib/option_quote.rs`, `src/ib/trade.rs`, `src/main.rs`.

## Blocking finding

1. `CLAUDE.md:3` — the PR changes prose outside the architecture-approved docs amendment.

   `arch.md` and the card both pin the docs work to the writes-gated sentence only: `AGENTS.md` + `CLAUDE.md` must copy `arch.md` §Docs amendment verbatim and change nothing else. The PR does make the required sentence replacement, but it also rewrites the `CLAUDE.md` intro from:

   - `it is the canonical agent-conventions doc: project map, the`
   - `**agent-first** authoring rule, hard safety rules, and how this repo is built.`

   to:

   - `canonical conventions: project map, **agent-first**`
   - `authoring, hard safety rules, build process.`

   That extra edit violates the semantic docs gate in `arch.md:16`, `arch.md:131`, and `tasks/01.md:59`. The journal notes the motivation was keeping `CLAUDE.md` under the frozen 900-byte limit, but the review contract still needs the scope change to be explicit rather than silently widening "nothing else".

   Required fix: either make the docs diff comply with the pinned amendment-only scope, or route a spec/architecture correction that explicitly permits the `CLAUDE.md` pointer trim because of the frozen `tests/claude_md.rs` byte budget. Do not hide additional prose rewrites inside this write-path PR.

## Semantic checks passed by reading

- `src/ib/trade.rs` keeps write containment: `rg -n "place_order|submit_order|encode_place_order|cancel_order" -S src` hits only `src/ib/trade.rs`.
- `place_core` preserves the safety ordering for gateway side effects: gate before connect, `client.next_order_id()` before `place_order`, and the first `OrderStatus`/`OpenOrder` is taken via `timeout_iter_data(TAKE_FIRST_TIMEOUT)`.
- Existing STK behavior is covered by the frozen stk suite, which stayed green; same validation messages/envelopes remain visible in the diff.
- `build_option_order` is LMT-only: `order_type = "LMT"`, `limit_price = Some(limit)`, TIF `Day`; no option MKT branch exists.
- No retry logic was introduced; timeout errors still describe UNKNOWN state and warn not to retry blindly.
- `src/ib/option_quote.rs` diff is exactly two visibility promotions: `normalize_right` and `parse_expiry` become `pub(crate)`.
- Option validation precedes gate/connect: right, expiry, finite-positive strike, finite whole-contract qty `>= 1`, and finite-positive limit all run before `place_core`.
- `shape_option_order_ack` emits exactly the 9 keys frozen by ADR 0020: `order_id`, `status`, `symbol`, `expiry`, `strike`, `right`, `action`, `quantity`, `limit_price`.
- `git diff --exit-code origin/main...origin/feat/option-orders -- Cargo.toml Cargo.lock` is empty; no new dependencies.
- `git diff --exit-code origin/main...origin/feat/option-orders -- src/ib/quote.rs src/ib/option_chain.rs src/ib/account.rs src/ib/portfolio.rs src/ib/positions.rs src/ib/orders.rs src/ib/completed_orders.rs src/ib/search.rs` is empty; read modules are untouched.
- Secret/account scan found no new credential or real account material in the PR diff; the only account-like value is an existing fake test fixture.

## Behavioral probes

All probes used `/tmp/omi-rev-oo-branch/target/debug/omi` with `env -u OMI_ALLOW_LIVE` and only dead-port or pre-gate live cases.

- Frozen matrix: `option-buy --live` and `option-sell --live` with valid args returned `code="config"`.
- Frozen matrix: hand-set `--port 4001` without `--live` returned `code="config"`.
- Frozen matrix: valid paper order against `127.0.0.1:65000` returned `code="connection"`.
- Frozen matrix: bad qty with `--live` returned `code="usage"`, proving usage validation precedes the live gate.
- Frozen matrix: missing `--limit` returned `code="usage"`.
- Adversarial: `--strike inf`, `--strike NaN`, and `--strike=-0` returned `code="usage"`.
- Adversarial: `--strike 2.5e2`, `--qty 2.0`, `--qty 1e15`, `--right call`, `--right put`, `--limit 1e-2`, and optional `--trading-class` all reached only `code="connection"` on the dead paper port.
- Adversarial: `--qty 1.5`, `--qty inf`, `--qty NaN`, `--qty=-0`, `--limit inf`, `--limit NaN`, `--limit=-0`, bad `--right X`, and dashed expiry all returned `code="usage"`.
