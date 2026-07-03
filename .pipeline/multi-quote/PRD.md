# PRD — multi-quote

Feature: variadic `omi quote SYM1 [SYM2 …]` — N snapshot quotes on ONE gateway connection,
input order, one JSON document. The brief-command consolidation win, mirrored onto the
market-data side of the daily flow.
Status: decision-complete (grilled 2026-07-03, operator locked D1–D3; code facts verified
against src/ib/quote.rs, src/cli.rs, and the frozen quote specs — not guessed).

## Problem

The daily market-side query — "how are my watchlist / held symbols doing" — costs one `omi quote
<SYM>` invocation PER symbol: N connect/handshake round-trips (the EAGAIN reconnect class,
`src/ib/mod.rs:40-50`), N `switch_market_data_type` calls, and N time-skewed snapshots the agent
must join. `omi brief` killed this exact waste on the account side; quotes are the remaining
N-invocation flow.

## Goal

`omi quote` accepts 1..N ticker symbols: connect ONCE, switch market-data type ONCE, fetch each
symbol's snapshot sequentially on that client (bounded SnapshotEnd drains), emit input-ordered
results, disconnect. N=1 stays byte-identical to today.

## Success criteria (acceptance)

1. **N=1 unchanged**: `omi quote AAPL` prints the SAME single JSON object as today
   (`{symbol, delayed, ticks{…}}`) — byte-identical on the same gateway state; every existing
   frozen spec stays green untouched.
2. **N≥2 → array**: `omi quote AAPL MSFT` prints a JSON array in INPUT order; each element is
   exactly the N=1 object shape (same keys, same `quote_price_tick` filtering, same `delayed`
   flag). No wrapper object (D2 preview-confirmed).
3. Exactly ONE gateway connection and ONE `switch_market_data_type` call per invocation,
   regardless of N.
4. **Fail-fast, no partial** (D3): any symbol's fetch failing ⇒ non-zero exit + structured
   envelope whose message NAMES the failing symbol, NOTHING on stdout.
5. Existing flags (`--sec-type`/`--exchange`/`--currency`, global `--md-type`) apply to the whole
   batch; STK-only guard unchanged (rejects before connecting).
6. Duplicate symbols pass through as given (no dedupe, order preserved) — the agent owns its
   input list.
7. `omi quote` with zero symbols ⇒ usage error (clap `num_args(1..)`), JSON usage envelope.
8. `cargo build` · `cargo clippy --all-targets -- -D warnings` · `cargo test` green; frozen
   quote specs (`tests/quote_ticks.rs`, quote lines in `tests/data_commands.rs`) untouched.
9. **Merge gate (operator, live):** `omi --live quote AAPL` and
   `omi --live quote AAPL MSFT NVDA` PASS in the same gateway session; the single-symbol run's
   object matches the batch run's corresponding row in shape (values may tick between reads).

## Scope

- `src/cli.rs`: `QuoteArgs.symbol: String` → `symbols: Vec<String>` (clap positional,
  `num_args(1..)`); help text stays truthful (still mentions md-type — frozen).
- `src/ib/quote.rs`: extract the per-symbol fetch into a seam (`quote_one`-style, taking
  `&Client` + symbol + the shared args) returning today's exact object; `quote()` becomes:
  STK guard → connect → switch md type once → loop symbols in order collecting rows →
  N=1 ⇒ the bare object, N≥2 ⇒ `Value::Array` (D2).
- Error contexts gain the symbol (e.g. `quote/MSFT`) for criterion 4.
- No new dependency, no new subcommand, no output.rs change (table renderer is generic).

## Non-scope (explicitly NOT this feature)

- No auto-sourcing symbols from positions/watchlist files — the agent composes its own list.
- No concurrency — sequential reads on one client (ADR 0010 discipline; snapshot drains are
  sub-second, N is small).
- No non-STK sec-types (Phase-1 guard stands), no new per-symbol flag overrides.
- No dedupe/normalization of the symbol list (criterion 6).
- No `--watch`/streaming; no retry semantics beyond existing connect-retry.

## Resolved decisions (locked)

- D1 **Feature choice = multi-symbol quote** (operator, grilled 2026-07-03). Picked over
  matching_symbols search (lower frequency), completed_orders (executions covers fills; flat
  account), scanner/news/option_chain (implementation weight / entitlement risk / phase
  mismatch). ROI: daily-frequency flow, N→1 connections, zero new API risk (snapshot path
  live-proven by quote; consolidation pattern proven by brief).
- D2 **Variadic `quote`, N=1 bare object / N≥2 bare array** (operator, preview-confirmed).
  Backward-compatible with every existing agent flow and the frozen dead-port test
  (`quote AAPL …` stays valid). The N-polymorphic shape is the accepted cost; the agent knows
  its own N. Rejected: a separate `quotes` subcommand (name bloat for the same mental model);
  always-array (breaks N=1 byte-identity — red line).
- D3 **Whole-command fail-fast** (operator). Any symbol failing kills the command with the
  symbol named; repo-wide no-partial rule (pnl-by-position/brief precedent). Agent degrades to
  per-symbol invocations.
- D4 **One switch, sequential bounded drains** (code). `switch_market_data_type` is
  per-connection (quote.rs:27-29) — called once before the loop; each symbol's snapshot drains
  to `SnapshotEnd` (bounded — NOT the markerless take-first class, no ADR 0012 timeout needed);
  consume-then-drop before the next request (ADR 0010 discipline).
- D5 **Unknown-symbol behavior = existing stream-error mapping** (code). The gateway surfaces a
  bad ticker as an error on the snapshot stream; it fail-fasts via the existing
  `market_data stream:` arm with the symbol-bearing context. Exact live behavior confirmed at
  acceptance (criterion 9) — no new classification logic.

## Risks / fragile assumptions

- **Back-to-back snapshot requests on one session** are new as a batch (pairwise prior art:
  brief interleaves 6 dataset kinds on one session — ADR 0010's routing analysis covers
  market_data's request-id domain). Arch verifies the snapshot cancel/cleanup on drop in
  ibapi-3.1.0 source; live acceptance (criterion 9) proves it on the Tiger gateway.
- Snapshot pacing: IB throttles market-data lines, but sequential snapshots release each line
  at SnapshotEnd before the next opens; N is agent-sized (≤ tens). Not a pacing hazard.
- A silent snapshot stream (never emits SnapshotEnd) would block a drain loop — the drain class
  is explicitly outside ADR 0012's take-first scope; no such wedge has ever been observed on
  quote. Recorded, not mitigated.
- Rollback: additive CLI change; revert restores single-symbol parsing.

## Verification

- Offline: frozen spec (task freezes; card-scoped runner) — the N-shaping seam (1 ⇒ object,
  ≥2 ⇒ array, order preserved), zero-symbol usage error, dead-port envelope with N symbols;
  existing quote specs untouched.
- Live (operator): criterion 9 — single + batch in one gateway session, row-shape cross-check.
