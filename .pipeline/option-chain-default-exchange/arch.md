# arch — option-chain-default-exchange

Stage: arch · feature: option-chain-default-exchange · decision of record: ADR 0028.
Read-only feature. The pure `shape_option_chain` seam is frozen and MUST NOT change.

## Chosen shape

reqSecDefOptParams is queried with NO server-side exchange filter; `--exchange` is applied client-side
via a new pure seam. Default `SMART` ⇒ one clean row.

## Data flow (option_chain gateway fn, src/ib/option_chain.rs)

```
connect(cfg)
  → contract_details(Contract::stock(sym))  → conid  (FIRST row, ADR 0019 D4)   [unchanged]
  → client.option_chain(sym, "", Stock, conid)                                    [CHANGED: "" not &args.exchange]
  → timeout-wrapped End-bounded drain → Vec<ChainRow>                             [unchanged, ADR 0016/0019]
  → filter_chain_rows(rows, &args.exchange)                                       [NEW pure seam]
  → shape_option_chain(sym, conid, rows) → JSON                                   [unchanged, FROZEN seam]
```

## Component boundaries / write-set for impl

- `src/ib/option_chain.rs`
  - NEW pure seam `pub fn filter_chain_rows(rows: Vec<ChainRow>, exchange: &str) -> Vec<ChainRow>`:
    `""` ⇒ passthrough; else retain `row.exchange == exchange` (exact-string, case-sensitive).
  - Gateway fn `option_chain`: pass `""` to `client.option_chain`; insert `filter_chain_rows` call
    between drain and `shape_option_chain`.
  - Update the module doc comment (line ~65 "`--exchange` is a server-side passthrough") + fn doc to
    describe client-side semantics.
  - `shape_option_chain` + `ChainRow`: UNTOUCHED.
- `src/ib/mod.rs`: add `filter_chain_rows` to the `pub use option_chain::{...}` re-export (so the frozen
  test can `use oh_my_ib::ib::filter_chain_rows`).
- `src/cli.rs` `OptionChainArgs.exchange`: default stays `"SMART"`; update the `///` help to
  "Client-side exchange filter; `SMART` (default) = consolidated view, `''` = all exchanges".
- NO write-path files. NO `trade.rs`. NO gate. This is a pure read path.

## Ordering / determinism

`filter_chain_rows` runs BEFORE `shape_option_chain` (which sorts rows by `(exchange, trading_class)`
and each row's expirations/strikes). Filtering by exchange is order-independent, so filter-then-sort is
observably identical to sort-then-filter (criterion 7). Fewer rows to sort is a minor bonus.

## Edge cases

- `--exchange SMART` on a gateway with no SMART row ⇒ `filter_chain_rows` returns `[]` ⇒
  `shape_option_chain` ⇒ `{"chains":[]}` (honest empty success — same path as today's zero-rows).
- `--exchange ""` ⇒ passthrough ⇒ all rows (Tiger: 20).
- Unknown `--exchange FOO` ⇒ `[]` (no match) ⇒ `chains: []`. Acceptable (agent mistyped an exchange).

## What task must freeze (anti-cheat)

- FREEZE (new red test, `tests/option_chain_command.rs` is the existing frozen file — a sibling test or
  a new file per task's choice): the pure `filter_chain_rows` seam — passthrough on `""`, exact-string
  retain on `<EX>`, empty on no-match, case-sensitivity, order preservation of the input subset. Build
  `ChainRow`s directly (the existing `row()` helper pattern) — offline, no gateway.
- NOT frozen (reviewed-by-reading + operator live acceptance, criteria 1–3): the gateway fn wiring
  (server call with `""`, seam insertion point) and the doc/help text — Tiger behavior is a journaled
  observation, never asserted in a test (the options-read card 01 precedent).
- Card `verify` MUST be card-scoped (a test-name filter over the new seam's tests), never the full suite
  (CONTRACT §Multi-card). `full-verify` in current.json stays `[cargo build, cargo test]`.

## Non-goals

Dedup of identical cross-exchange rows; option-quote; any server-side retry/fallback; timeout/drain
posture changes (reused verbatim).
