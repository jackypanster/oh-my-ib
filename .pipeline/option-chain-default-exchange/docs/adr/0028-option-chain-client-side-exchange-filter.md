# ADR 0028 — option-chain: client-side `--exchange` filter

Status: accepted · 2026-07-06 · feature: option-chain-default-exchange · supersedes the server-side
`--exchange` passthrough described in ADR 0019 / option_chain.rs for the exchange dimension only.

## Context

`omi option-chain <SYM>` is empty out-of-box on the Tiger gateway. reqSecDefOptParams
(`client.option_chain(sym, exchange, Stock, conid)`) takes a server-side `exchange` argument; the CLI
defaults it to `"SMART"`. LIVE-verified on Tiger `:4001` (2026-07-06, acct U20230856):

- `exchange="SMART"` (default) ⇒ **zero rows** (`{"chains":[]}`).
- `exchange=""` (all) ⇒ **20 rows**, one per physical exchange **including a `SMART` row**, and all 20
  are **content-identical** (one distinct `(expirations, strikes)` signature).

So Tiger's server-side exchange filter is **unreliable**: it drops the SMART view that demonstrably
exists in the unfiltered result. The default is therefore dead, and the only working invocation
(`--exchange ""`) is 20× redundant.

## Decision

**Move the exchange filter client-side.**

1. Call reqSecDefOptParams with an **empty** server-side exchange (`""`) ALWAYS — bypass Tiger's broken
   server filter and fetch the full row set reliably.
2. Introduce a **pure, frozen-testable seam**:
   `pub fn filter_chain_rows(rows: Vec<ChainRow>, exchange: &str) -> Vec<ChainRow>`
   - `exchange == ""` ⇒ passthrough (all rows).
   - else ⇒ retain rows where `row.exchange == exchange` (**exact-string, case-sensitive**).
3. Apply it in the `option_chain` gateway fn **between the drain and `shape_option_chain`**.
4. The CLI `--exchange` default stays the literal `"SMART"`; only its **meaning** changes
   (server passthrough → client-side filter). Out-of-box ⇒ keep only the `SMART` row ⇒ 1 clean row.

## Consequences

- `omi option-chain AAPL` (no flag) ⇒ the single `SMART` row (clean, non-redundant). [criterion 1]
- `omi option-chain AAPL --exchange ""` ⇒ all rows (unchanged from today). [criterion 2]
- `omi option-chain AAPL --exchange AMEX` ⇒ only the AMEX row. [criterion 3]
- `--exchange SMART` on a gateway that emits NO SMART row ⇒ `filter_chain_rows` returns empty ⇒
  `shape_option_chain` returns `{"chains":[]}` — honest empty, not a crash. [criterion 4]
- The pure `shape_option_chain` seam is **UNTOUCHED** (its frozen test stays green); the filter is a
  SEPARATE seam. `filter_chain_rows` is frozen-tested by this feature's task stage.
- Filter is order-independent, so filter-before-sort == sort-before-filter; observable output is
  identical to a post-sort filter. [criterion 7]
- `option-quote` is unaffected — its `--exchange` is a *routing* exchange on the option Contract for the
  MD snapshot (part of its frozen 8-key contract echo), not a reqSecDefOptParams filter. OUT of scope.

## Alternatives rejected

- **Default `--exchange ""` (server-side, all exchanges).** Non-empty but 20× redundant on Tiger; pushes
  dedup onto every caller. Rejected — poor agent-facing default.
- **Keep server-side `SMART` + auto-retry `""` on zero rows.** Magic empty-retry, an extra round-trip on
  every Tiger call, and STILL yields 20 redundant rows. Conflates "no options" with "SMART filter empty".
  Rejected.
- **Client-side dedup/collapse of identical cross-exchange rows.** Separate concern; out of scope (the
  `--exchange ""` caller opts into all rows knowingly).
