# PRD — option-chain-default-exchange

Stage: prd · feature: option-chain-default-exchange · repo: jackypanster/oh-my-ib · branch: main
Read-only feature (no write path). Author: cc (Claude). HITL decision recorded below.

## Problem

`omi option-chain <SYM>` returns empty out-of-box on the Tiger gateway — the discovery step of the
whole option read/quote/order flow is dead by default.

Root cause (LIVE-verified 2026-07-06, Tiger `:4001`, acct U20230856, via `target/debug/omi`):

- CLI default is `--exchange SMART` (`src/cli.rs` `OptionChainArgs.exchange` `default_value="SMART"`),
  passed **server-side** to reqSecDefOptParams (`src/ib/option_chain.rs:89`
  `client.option_chain(&args.symbol, &args.exchange, SecurityType::Stock, conid)`).
- **Tiger's server-side exchange filter is broken**: passing `"SMART"` returns ZERO rows even though a
  `SMART` row EXISTS in the unfiltered result.
- Evidence:
  - `omi --live option-chain AAPL` → `{"chains":[]}`
  - `omi --live option-chain AAPL --exchange ""` → **20 rows** (AMEX, BATS, BOX, CBOE, CBOE2, EDGX,
    EMERALD, GEMINI, IBUSOPT, ISE, MEMX, MERCURY, MIAX, NASDAQBX, NASDAQOM, PEARL, PHLX, PSE, SAPPHIRE,
    **SMART**), **all content-identical** — 1 distinct `(expirations, strikes)` signature across all 20.
- So out-of-box the agent cannot discover expirations/strikes; `--exchange ""` "works" but is 20×
  redundant (20 identical copies).

## Goal

Out-of-box `omi option-chain <SYM>` (no `--exchange` flag) returns a **non-empty, non-redundant** chain
— the canonical SMART consolidated view (one row of expirations+strikes) — deterministic, READ-ONLY,
JSON envelope/keys UNCHANGED.

## Decision (operator-confirmed, HITL)

**Move the exchange filter CLIENT-SIDE.** reqSecDefOptParams is queried with NO server-side exchange
filter (reliable — returns all rows on Tiger); `--exchange` becomes a **client-side** filter on the
returned rows:

- default `--exchange SMART` → keep only the `SMART` row → **1 clean row** (agent gets one copy of
  expirations+strikes).
- `--exchange ""` → all rows (no client filter) — same as today's `--exchange ""` (20 rows on Tiger).
- `--exchange <EX>` (e.g. `AMEX`) → only that exchange's row(s).

Rationale: Tiger's server-side filter is unreliable (drops SMART despite it existing); a client-side
filter on the full row set is reliable, preserves the intuitive "SMART = default consolidated view",
and needs no magic empty-retry.

## Success criteria (live acceptance on Tiger `:4001` + offline frozen)

1. `omi --live option-chain AAPL` (no `--exchange`) → `chains` = exactly the **SMART** row (non-empty),
   carrying the real expirations + strikes. [live]
2. `omi --live option-chain AAPL --exchange ""` → **all** exchange rows (unchanged from today). [live]
3. `omi --live option-chain AAPL --exchange AMEX` → only the AMEX row. [live]
4. `--exchange SMART` (or any `<EX>`) when the gateway returns NO matching row → `chains: []` success
   (honest empty; the client filter found nothing) — documented, not a crash. [offline seam]
5. JSON envelope keys UNCHANGED: `{underlying, conid, chains[].{exchange, trading_class, multiplier,
   expirations, strikes}}`. The pure `shape_option_chain` seam is **UNTOUCHED** — existing frozen test
   `tests/option_chain_command.rs` stays green. [offline]
6. READ-ONLY: no write path touched. `option-quote` is OUT of scope (its `--exchange SMART` default is a
   *routing* exchange on the quote contract — semantically correct smart-routing, a different usage).
7. Deterministic ordering preserved (rows sorted by `(exchange, trading_class)`; the filter runs before
   or after sort with identical observable result).
8. `cargo build` + full suite (`cargo test`) green; `cargo clippy --all-targets -- -D warnings` clean.

## Scope

- IN: `--exchange` semantics server-side → client-side; the gateway fn `option_chain` in
  `src/ib/option_chain.rs`; the CLI `--exchange` help text (now "client-side filter; `SMART`=default
  consolidated view, `''`=all exchanges").
- OUT: `option-quote` (different, correct usage); the pure `shape_option_chain` seam (unchanged); any
  write path; dedup/collapse of the 20 identical rows under `--exchange ""` (caller opts into all-
  exchanges knowingly).

## Non-scope / explicit omissions

- NOT collapsing/deduping identical cross-exchange rows under `--exchange ""` — separate concern.
- NOT changing the timeout/drain posture (ADR 0016/0019 reused verbatim).
- NOT adding a server-side retry/fallback — the client-side filter replaces the server filter outright.

## Open design (for arch to resolve + ADR)

- WHERE the client filter lands: prefer a NEW **pure, frozen-testable** seam
  `filter_chain_rows(rows, &exchange) -> rows` (offline-testable with no gateway; `""` ⇒ passthrough,
  `<EX>` ⇒ retain `row.exchange == EX`), applied in the gateway fn between drain and `shape_option_chain`.
  arch confirms the seam name/signature + whether the match is exact-string case-sensitive.
- reqSecDefOptParams call: pass `""` (all exchanges) ALWAYS to `client.option_chain`, since Tiger's
  server filter is unreliable. arch confirms.
- Update the module doc comment (`option_chain.rs:65` "`--exchange` is a server-side passthrough") and
  the fn doc to reflect the client-side semantics.

## Gotchas (project-specific)

- `shape_option_chain` must stay byte-identical — the new filter is a SEPARATE seam/step, never a shape
  edit. The existing frozen test asserts the shape; the gateway fn's old "server-side passthrough" was
  NOT frozen (reviewed-by-reading per options-read card 01), so changing it is in-bounds.
- `--exchange` default stays the literal string `"SMART"` in `cli.rs`; only its MEANING changes.
- Tiger emits a `SMART` row in the unfiltered set — the exact-string client filter `== "SMART"` is what
  yields the clean default. Confirm case-sensitivity against the live row (`SMART`, upper).
- Live acceptance requires the Tiger gateway open on `:4001` (Read-Only API state irrelevant — reads
  work either way) and `--exchange ""` to have been observed returning a SMART row (it does today).

## Verify

`cargo build` · `cargo test` (full suite) · `cargo clippy --all-targets -- -D warnings` · operator live
acceptance criteria 1–3 on Tiger `:4001`.
