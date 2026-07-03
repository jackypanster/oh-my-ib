# PRD — search-command

Feature: `omi search <pattern>` — fuzzy symbol/company lookup via TWS `reqMatchingSymbols`,
returning the gateway's match list (conid, symbol, sec type, exchange, currency) as a JSON
array. The missing first step of every "look up X, then quote/contract/history it" agent flow.
Status: decision-complete (grilled 2026-07-03, operator locked D1–D3; ibapi facts verified in
crate source, not guessed).

## Problem

`omi contract` / `omi quote` / `omi history` all require the EXACT ticker symbol. When the
agent starts from a company name ("Apple"), a partial symbol, or an unfamiliar market's
ticker, there is no way to resolve it inside `omi` — the agent falls back to guessing or
external search. IB ships exactly this primitive (`reqMatchingSymbols`), unused so far.

## Goal

New read-only subcommand `omi search <pattern>`: connect, send ONE matching-symbols request,
emit the gateway's matches as a JSON array (pass-through, agent filters), disconnect. The
simplest read class in the repo: ibapi's `matching_symbols(pattern)` returns `Vec<ContractDescription>`
directly — single request/response, no subscription lifecycle in our code
(contracts/sync.rs:143-155, source-verified).

## Success criteria (acceptance)

1. `omi search apple` (paper default) exits 0 and prints a JSON ARRAY of match rows, in the
   gateway's order (pass-through, no re-sorting).
2. Each row carries the contract identity fields the gateway returns — expected keys (arch pins
   the exact set from ibapi's `decode_contract_descriptions`): `conid`, `symbol`, `sec_type`,
   `primary_exchange`, `currency`, `derivative_sec_types` (array). Rows are transparent
   pass-through (D3): non-STK / non-US rows included, no filtering.
3. Zero matches ⇒ `[]`, exit 0 (not an error).
4. Multi-word patterns work as ONE quoted positional (`omi search "hong kong"`); a missing
   pattern ⇒ usage envelope (clap required).
5. Gateway down ⇒ existing connection-error contract; gateway-side request error ⇒ existing
   `data` envelope.
6. `--format table` renders rows via the existing generic renderer (no output.rs change).
7. `cargo build` · `cargo clippy --all-targets -- -D warnings` · `cargo test` green; all
   existing frozen specs untouched.
8. **Merge gate (operator, live):** `omi --live search apple` returns a non-empty array
   containing an AAPL row; one non-US/CJK-adjacent pattern spot-checked (informational).

## Scope

- `src/cli.rs`: `Search(SearchArgs)` variant + `SearchArgs { pattern: String }` (single
  required positional).
- `src/ib/search.rs` (new): connect → `client.matching_symbols(&pattern)` → shape rows →
  emit array. Pure shaping seam (ContractDescription-free row builder) for the frozen spec,
  mirroring the `shape_*` house pattern.
- `src/ib/mod.rs`: `mod search;` + re-exports. `src/main.rs`: dispatch arm.
- No new dependency.

## Non-scope (explicitly NOT this feature)

- No sec-type/exchange filter flags (D3: agent filters client-side; zero filter logic).
- No pagination/limit flags — the gateway caps matches (~16) itself; pass-through.
- No auto-chaining (search → contract details) — compose with `omi contract` / `omi quote`.
- No Phase-1 STK guard here: search is read-only METADATA, not a market-data/trading request;
  the STK-only gate stays on `quote` (grilled, D3 rationale).
- No fuzzy-ranking/scoring of our own.

## Resolved decisions (locked)

- D1 **Feature choice = symbol search** (operator, grilled 2026-07-03). Picked over
  account_summary-tags (account/brief cover it), completed_orders (executions covers fills;
  flat account), historical extras (niche), news/scanner/option_chain (entitlement /
  implementation weight / phase mismatch). ROI: unblocks every symbol-resolution flow at the
  lowest implementation cost in the repo.
- D2 **Name = `omi search`** (operator). Verb, unambiguous, no collision; `lookup` implies
  exact-match semantics this API doesn't have; `symbols` collides with quote's SYMBOL args.
- D3 **Full pass-through rows** (operator). All sec types / markets included with identifying
  fields per row; the agent filters. STK-only is a market-data/trading gate, not a metadata
  gate.
- D4 **Single positional pattern** (code). One string, shell-quoted for spaces — matches the
  upstream API (`matching_symbols(pattern: &str)`).
- D5 **No subscription lifecycle** (code). The crate returns `Vec<ContractDescription>`
  directly (single `SymbolSamples` message, request-id domain); our code maps `Err` →
  `AppError::data` and shapes rows. NOT the take-first class (no ADR 0012 timeout), NOT a
  drain loop — a plain bounded call.

## Risks / fragile assumptions

- **IB rate-limits `reqMatchingSymbols` (~1/sec).** One request per invocation ⇒ irrelevant
  unless an agent loops; recorded as an operational note, no client-side limiter (YAGNI).
- Exact row fields depend on server version (`decode_contract_descriptions` — some fields
  arrive only on newer servers). Arch pins the emitted key set from the decoder source so the
  shape is deterministic regardless (absent → null/[] per house sentinel rules).
- Live match quality (does "apple" return AAPL on this gateway/entitlement?) is
  gateway-decided — criterion 8 verifies live; offline spec freezes only the pure shaping.
- Rollback: purely additive subcommand.

## Verification

- Offline: frozen spec — pure row-shaping seam (field mapping, empty → `[]`, order
  preserved), CLI contract (help lists search; missing pattern ⇒ usage; dead port ⇒
  connection envelope).
- Live (operator): criterion 8.
