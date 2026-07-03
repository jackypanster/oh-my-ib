# Review 01 — options-read

Verdict: REJECT

## Gate evidence

- Freeze gate: PASS. Command run: `git diff 7c8bcaf5 origin/feat/options-read -- tests/option_chain_command.rs tests/option_quote_command.rs`; output empty.
- Full suite on detached PR head `/tmp/omi-review-branch` at `0580dda`: PASS.
  - `cargo build`: pass.
  - `cargo test`: pass, 139 tests across 21 suites.
  - `cargo clippy --all-targets -- -D warnings`: pass.
- PR surface: `gh pr diff 16 --name-only` shows `AGENTS.md`, `CLAUDE.md`, `src/cli.rs`, `src/ib/mod.rs`, `src/ib/option_chain.rs`, `src/ib/option_quote.rs`, `src/main.rs`.

## Blocking finding

1. `src/ib/option_quote.rs:162` — `--strike NaN` and `--strike inf` bypass the pre-connect validation.

   Trigger: run from the detached PR head:

   - `target/debug/omi --format json option-quote --symbol AAPL --expiry 20260918 --strike NaN --right C --host 127.0.0.1 --port 65000`
   - `target/debug/omi --format json option-quote --symbol AAPL --expiry 20260918 --strike inf --right C --host 127.0.0.1 --port 65000`

   Actual: both return `{"error":{"code":"connection",...}}`, proving the command attempted to connect.

   Expected: `usage` before any connection attempt. Card 02 requires pre-connect validation for `strike > 0`; `NaN` is not greater than zero, and infinity is not a valid option strike. The implementation only checks `args.strike <= 0.0`, which is false for `NaN` and also lets positive infinity through.

   Required fix: reject non-finite strikes before `super::connect(cfg)?`, for example `if !args.strike.is_finite() || args.strike <= 0.0 { usage }`. Keep `tests/option_quote_command.rs` frozen; add source-local coverage only if the impl path wants an extra regression.

## Semantic checks passed by reading

- `src/ib/option_chain.rs:81` uses the FIRST `contract_details` row for conid; empty details return `not_found`.
- `src/ib/option_chain.rs:89` passes `args.exchange` directly to `client.option_chain`, so `--exchange` is server-side passthrough.
- `src/ib/option_chain.rs:93` uses `timeout_iter_data(super::TAKE_FIRST_TIMEOUT)` and `src/ib/option_chain.rs:112` / `:121` implement the ADR 0016 Instant-classified `None` arms.
- `src/ib/option_quote.rs:156-176` validates right/strike/expiry before `src/ib/option_quote.rs:178` connects, except for the non-finite strike bug above.
- `src/ib/option_quote.rs:191-202` builds with `Contract::call`/`put`, `.strike`, `.expires_on`, `.on_exchange`, `.in_currency`, and optional `.trading_class`.
- `src/ib/option_quote.rs:213-217` uses bare `iter_data()` to `SnapshotEnd`; no timeout wrapper.
- `src/ib/option_quote.rs:222-223` makes model greeks last-row-wins.
- Read-only polarity: source grep for `place_order|submit_order|encode_place_order|cancel_order` under `src/` hits only `src/ib/trade.rs`; `git diff --name-only origin/main...HEAD -G 'place_order|submit_order|encode_place_order|cancel_order'` is empty.
- `git diff --exit-code origin/main...HEAD -- Cargo.toml Cargo.lock src/ib/quote.rs src/ib/trade.rs` is empty: no dependency changes, `quote.rs` byte-untouched, `trade.rs` untouched.
- Docs amendment: PR diff makes the two required `AGENTS.md` edits and the `CLAUDE.md` writes-gated sentence edit, matching `arch.md` content.
- Secrets/account ids: diff grep found no account IDs, API keys, tokens, or credentials introduced by this PR.

## Adversarial notes

- Empty chains are frozen and pass (`tests/option_chain_command.rs:123-130`).
- NaN chain strikes: `shape_option_chain` follows the pinned `partial_cmp` approach and will not panic; IB option-chain strikes are expected finite contract strikes, so this is not a blocker for card 01.
- All-None model greeks row: output becomes `"greeks": {}`. That matches ADR 0019 D3 because a model row did arrive and fields are omit-if-None.
