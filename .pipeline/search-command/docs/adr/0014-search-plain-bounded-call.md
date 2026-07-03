# ADR 0014 — `omi search`: the plain-bounded-call read class, full pass-through rows

Status: accepted

## Context

Every existing gateway read falls into one of two stream classes: **drain-to-End** (account,
positions, orders, executions, quote's SnapshotEnd) or **markerless take-first** (pnl,
pnl_single — bounded by ADR 0012). `reqMatchingSymbols` is neither: ibapi-3.1.0's
`matching_symbols(pattern)` sends one request, reads one `SymbolSamples` message, and returns
`Vec<ContractDescription>` directly (contracts/sync.rs:143-155) — no subscription object ever
reaches caller code.

## Decision

1. `omi search <pattern>` (single required positional; shell quoting for multi-word) calls
   `matching_symbols` once and emits the matches as a JSON array via the pure frozen seam
   `shape_search` — **gateway order, full pass-through** (operator D3): all sec types and
   markets included, 7 identity keys per row (`conid`, `symbol`, `sec_type`,
   `primary_exchange`, `currency`, `description`, `derivative_sec_types`).
2. **No STK guard** (operator D3): Phase-1's STK-only gate protects market-data/trading
   requests; search is read-only metadata. `quote` keeps its guard; `search` must not copy it.
3. **Plain bounded call class**: no ADR 0012 timeout (not markerless), no drain loop, no
   cancel/Drop concerns. Errors map to the existing `data` envelope (context `search`).
4. Strings pass through as-is ("" stays "") — the `pnl_number` sentinel rule is for money
   `f64` only; nothing here is money. Decoder defaults (`unwrap_or_default`,
   proto/decoders.rs:84-108) make the shape total on every server version.

## Rationale

- Simplest read in the repo — the crate already owns the request/response lifecycle; our code
  is field mapping only. Zero entitlement risk, zero pacing exposure per invocation.
- Pass-through beats filtering: a filtered-empty result ("no STK matches") is
  indistinguishable from "no matches" and misleads the agent; identity fields per row let the
  agent filter with full information.
- Rejected: sec-type/exchange filter flags (client-side agent concern, PRD non-scope);
  own ranking/scoring (gateway order is the contract); `lookup`/`symbols` naming (exact-match
  connotation / SYMBOL-arg collision).

## Consequences

- Agents resolve fuzzy names to conid/symbol inside `omi`, then chain to
  `contract`/`quote`/`history` themselves.
- IB rate-limits `reqMatchingSymbols` (~1/sec, IB-side). One request per invocation makes this
  moot; recorded so a future batch/looping idea starts from this constraint.
- The result set size is gateway-capped (~16); no pagination surface.
- A third read class ("plain bounded call") now exists as prior art for future
  request-response endpoints (market_rule, option_chain metadata, …).
