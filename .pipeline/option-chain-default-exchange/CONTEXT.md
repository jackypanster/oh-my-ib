# CONTEXT — option-chain-default-exchange

Domain glossary + conventions for this feature. Grounded in the codebase, not invented.

## Terms

- **reqSecDefOptParams** — the IB/TWS API request behind `option-chain`; ibapi surfaces it as
  `client.option_chain(symbol, exchange, sec_type, conid)`, streaming `SecurityDefinitionOptionParameter`
  rows terminated by `...End`. Returns the discovery set (expirations × strikes) per exchange × trading
  class. See `src/ib/option_chain.rs`, ADR 0019 (drain/timeout posture).
- **Server-side exchange filter** — the `exchange` argument passed to reqSecDefOptParams; the *gateway*
  restricts which exchange rows it returns. **Unreliable on Tiger** — `"SMART"` returns nothing though a
  SMART row exists unfiltered (the bug this feature fixes).
- **Client-side exchange filter** — filtering the fully-returned row set locally by `row.exchange`
  (new seam `filter_chain_rows`). Reliable because it does not depend on the gateway honoring the filter.
  This feature moves the filter from server-side to client-side.
- **SMART row** — the consolidated smart-routed view of the chain. On Tiger it appears in the unfiltered
  (`exchange=""`) result alongside ~19 physical-exchange rows (AMEX, CBOE, PHLX, …), all content-
  identical. It is the canonical single-row answer an agent wants from `option-chain`.
- **ChainRow** — the plain, ibapi-free chain row (`{exchange, trading_class, multiplier, expirations,
  strikes}`); the frozen test constructs these directly. The unit both the shape seam and the new filter
  seam operate on.
- **shape_option_chain** — the FROZEN pure seam: `rows → {underlying, conid, chains:[...]}`, sorts for
  determinism, `[] ⇒ chains:[]`. UNTOUCHED by this feature.
- **filter_chain_rows** — the NEW pure seam: `(rows, exchange) → rows`. `"" ⇒ all`; else exact-string
  case-sensitive retain on `row.exchange`. Frozen-tested by this feature.

## Conventions (from the repo)

- Pure seams are offline-testable (build `ChainRow`/rows directly), gateway fns are reviewed-by-reading +
  operator live acceptance (Tiger behavior is never asserted in a test — options-read card 01 precedent).
- Read path only: no `trade.rs`, no write gate, no `OMI_ALLOW_LIVE`. Live reads work regardless of the
  gateway's Read-Only API toggle.
- `--exchange` default in `cli.rs` stays the literal string `"SMART"`; the feature changes its SEMANTICS,
  not its default value.
- JSON envelope keys are a stable machine contract — unchanged by this feature.
