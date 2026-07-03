# arch — search-command

How `omi search` lands. Binding decisions in **ADR 0014**; glossary in `CONTEXT.md`. All ibapi
claims verified in the vendored crate source.

## Design shape (four touched files, no new dependency)

| file | change |
|---|---|
| `src/cli.rs` | `Search(SearchArgs)` variant (doc: "Fuzzy symbol/company search") + `SearchArgs { pattern: String }` (single required positional, doc "Search text, e.g. apple or \"hong kong\"") |
| `src/ib/search.rs` | NEW — plain `SearchRow` struct, pure frozen seam `shape_search`, gateway fn `search` |
| `src/ib/mod.rs` | `mod search;` + `pub use search::{search, shape_search, SearchRow};` |
| `src/main.rs` | `Command::Search(args) => ib::search(&config, args),` dispatch arm |

NOT touched: `output.rs` (generic renderer), `error.rs`, every other `ib/` module, all tests.

## ibapi facts (source-verified, 2026-07-03)

- `client.matching_symbols(pattern) -> Result<Vec<ContractDescription>, Error>` — sends ONE
  `reqMatchingSymbols`, reads ONE `SymbolSamples` message, decodes, returns the Vec
  (contracts/sync.rs:143-155). No subscription object reaches our code: neither take-first
  (no ADR 0012 timeout) nor drain-to-End — a **plain bounded call** (ADR 0014 class).
- Row source: `ContractDescription { contract: Contract, derivative_security_types: Vec<String> }`
  (contracts/mod.rs:914-919); `decode_contract` populates from proto with
  `unwrap_or_default()` — absent fields become `0`/`""`/`[]`, never panic
  (proto/decoders.rs:84-108). Fields meaningful on SymbolSamples: `contract_id`, `symbol`,
  `security_type`, `primary_exchange`, `currency`, `description` (company name, may be empty
  on older servers).
- Newtype wrappers (`Symbol`, `Exchange`, `Currency`, `SecurityType`) render via
  `.to_string()`.

## Component design (impl follows this verbatim)

`src/ib/search.rs`:

```rust
/// Plain, ibapi-free match row (the frozen test constructs these directly — PnlSingleRow pattern).
pub struct SearchRow {
    pub conid: i32,
    pub symbol: String,
    pub sec_type: String,
    pub primary_exchange: String,
    pub currency: String,
    pub description: String,          // company name; "" when the gateway omits it
    pub derivative_sec_types: Vec<String>,
}

/// The pure, FROZEN shaping seam: rows in gateway order → JSON array (PRD D3 pass-through).
/// Exactly 7 keys per row; strings pass through as-is ("" stays "", house pass-through rule —
/// the pnl_number sentinel applies to money f64 only, nothing here is money). Empty ⇒ json!([]).
pub fn shape_search(rows: Vec<SearchRow>) -> Value {
    // per-row: json!({ "conid", "symbol", "sec_type", "primary_exchange", "currency",
    //                  "description", "derivative_sec_types" })
}

pub fn search(cfg: &Config, args: &SearchArgs) -> Result<Value, AppError> {
    let client = super::connect(cfg)?;
    let matches = client.matching_symbols(&args.pattern)
        .map_err(|e| AppError::data(format!("matching_symbols failed: {e}"), "search"))?;
    let rows = matches.into_iter().map(|d| SearchRow {
        conid: d.contract.contract_id,
        symbol: d.contract.symbol.to_string(),
        sec_type: d.contract.security_type.to_string(),
        primary_exchange: d.contract.primary_exchange.to_string(),
        currency: d.contract.currency.to_string(),
        description: d.contract.description.clone(),
        derivative_sec_types: d.derivative_security_types,
    }).collect();
    Ok(shape_search(rows))
}
```

No account resolution (search is account-independent), no STK guard (PRD D3 — metadata, not
market-data; do NOT copy quote's guard), no md-type switch.

## Freeze coverage (pinned for pipeline-task)

- **Frozen (`tests/search_command.rs`):** `shape_search` pure seam (exact 7-key row set;
  gateway order preserved; "" description passes through; empty derivative list ⇒ `[]` key
  present; zero rows ⇒ `json!([])`); CLI (`--help` lists `search`; `search --help` ok; missing
  pattern ⇒ `code="usage"`; dead port ⇒ `code="connection"`). House-red via
  `use oh_my_ib::ib::{shape_search, SearchRow};`.
- **Review-by-reading:** the gateway fn (one call, field mapping per above, `data` error
  context "search", no guard/no timeout).
- **Live (operator, PRD criterion 8, merge gate):** `omi --live search apple` non-empty, AAPL
  row present; one non-US pattern spot-checked (informational).

## Risks re-checked

- Rate limit (~1 reqMatchingSymbols/sec, IB-side): one per invocation — no client limiter
  (YAGNI; ADR 0014 records the note for future batch ideas).
- Server-version field variance: decoder defaults make the shape total; `description` may be
  "" — frozen test pins the pass-through.
- `pattern` containing only whitespace: passed through to the gateway verbatim (gateway
  decides); no client-side validation beyond clap's required arg.
