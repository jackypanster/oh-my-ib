# ADR 0013 — Batch snapshot quotes on one session, N-polymorphic output

Status: accepted

## Context

Each `omi quote <SYM>` invocation costs a full connect/handshake (EAGAIN reconnect class,
`src/ib/mod.rs:40-50`) and its own `switch_market_data_type`. The daily watchlist flow pays this
N times and gets N time-skewed snapshots. brief-command (ADR 0010/0011) already proved the
consolidation pattern on the account side; this ADR applies it to market data.

## Decision

1. **Variadic `quote`** (`symbols: Vec<String>`, clap `required = true`): connect ONCE, switch
   market-data type ONCE, then fetch each symbol's snapshot **sequentially, in input order**,
   consume-to-`SnapshotEnd`-then-drop per symbol (ADR 0010 discipline).
2. **Per-symbol seam `quote_one`** emits exactly the pre-existing single-symbol object
   (`{symbol, delayed, ticks{…}}`); error contexts become `quote/<symbol>`.
3. **N-polymorphic output via the pure frozen seam `shape_quotes`**: 1 row ⇒ bare object
   (byte-identical to the pre-variadic output), 2+ ⇒ bare array in input order, 0 ⇒ `[]`
   (defensive, unreachable via CLI).
4. **Whole-command fail-fast** (operator D3): the first failing symbol aborts with a
   symbol-naming error; no partial output.

## Rationale

- Safety of the batch is a routing fact, not hope: `market_data()` allocates a fresh
  request-id per call (request-id routing domain, market_data/realtime/sync.rs:186-197), and
  drop sends `CancelMarketData(request_id)` (realtime/mod.rs:379) — with consume-then-drop at
  most one market-data line is open at any moment, so no cross-talk and no pacing exposure.
- `switch_market_data_type` is connection-level (send_shared_request, sync.rs:176-182): once
  before the loop is both sufficient and the only correct placement.
- N-polymorphism (object vs array) was operator-chosen over a separate `quotes` subcommand
  (name bloat) and over always-array (breaks the N=1 byte-identity red line). The agent knows
  its own N.
- Snapshot drains are `SnapshotEnd`-bounded — deliberately NOT wrapped in ADR 0012's take-first
  timeout (that mechanism is for MARKERLESS streams only; blanket timeouts were explicitly not
  licensed by ADR 0012).

## Consequences

- Watchlist flow: N connections → 1; one `switch_market_data_type`; time-consistent batch.
- **Accepted N=1 delta, failure path only**: error CONTEXT strings gain the symbol
  (`"quote"` → `"quote/AAPL"`). Codes/messages/success output unchanged; frozen tests assert
  `code`, not `context`. Recorded so review doesn't flag it as drift.
- A symbol list with duplicates yields duplicate rows (input order, no dedupe) — the agent owns
  its list.
- If a snapshot stream ever fails to emit `SnapshotEnd` live, the drain blocks — same posture
  as every drain-to-End command; evidence-gated future work, not this feature (no such wedge
  observed on quote in any live acceptance to date).
