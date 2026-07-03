# CONTEXT — search-command

New domain terms. Base glossary: `.pipeline/phase1-readonly/CONTEXT.md`. Only deltas here.

## Search domain

- **Plain bounded call** — the third gateway-read class (ADR 0014): one request, one decoded
  response, the crate returns a `Vec` directly. Neither drain-to-End nor take-first; no
  subscription lifecycle, no timeout wrapping, no cancel concerns.
- **Match row / `SearchRow`** — plain ibapi-free struct (PnlSingleRow pattern): `conid`,
  `symbol`, `sec_type`, `primary_exchange`, `currency`, `description` (company name, "" when
  the gateway omits it), `derivative_sec_types` (string array).
- **`shape_search`** — the pure FROZEN seam: rows in gateway order → JSON array; exact 7-key
  rows; empty ⇒ `[]`. Full pass-through (D3): no sec-type/market filtering, no re-ranking.
- **Gateway order** — the result order IS the contract; `omi` never re-sorts matches.

## Conventions (feature-specific)

- `omi search apple` / `omi search "hong kong"` — ONE positional pattern, shell-quoted for
  spaces. Missing pattern ⇒ usage envelope.
- NO STK guard (metadata read, not market-data — D3); no account resolution; no md-type
  switch. Do not copy `quote`'s preamble.
- Strings pass through as-is ("" stays ""); `pnl_number` sentinel rules do NOT apply (no money
  fields).
- Zero matches ⇒ `[]`, exit 0. Gateway request error ⇒ `data` envelope, context `search`.
- IB rate-limits reqMatchingSymbols ~1/sec (IB-side); one request per invocation.
- **Merge gate (PRD criterion 8)**: operator live-accepts `omi --live search apple` (AAPL row
  present) before the PR merges.
