# CONTEXT — snapshot-timeout

Deltas only. Base glossary: `.pipeline/phase1-readonly/CONTEXT.md`; reuses **timeout twin** /
**TAKE_FIRST_TIMEOUT** / **cure message** (read-timeouts), **Instant-classified None** (ADR 0016),
**snapshot drain** (multi-quote ADR 0013, options-read ADR 0019).

- **Snapshot drain** — `market_data(..).snapshot().subscribe()` consumed until `TickTypes::SnapshotEnd`.
  Two sites only: `quote_one` (`src/ib/quote.rs`), `option_quote` (`src/ib/option_quote.rs`).
- **Silent snapshot** — the live-proven failure (2026-09-26, IB Gateway 10.45, market closed, live
  md-type): the gateway accepts the snapshot request and then sends nothing — no tick, no error, no
  SnapshotEnd. Distinct from a **wedge** (read-timeouts): no orphan pollution observed, no restart cure.
- **IB snapshot tail** — IB's documented ≈11s from request to tickSnapshotEnd (📖 TWS API docs).
- **`SNAPSHOT_DEADLINE`** — ONE shared pub const, 20s, `src/ib/mod.rs`; a TOTAL deadline per snapshot
  request (not per item). Must be > the 11s IB tail; therefore ≠ `TAKE_FIRST_TIMEOUT` (10s).
- **Total-deadline loop** — `next_timeout(deadline − now)` repeated until SnapshotEnd / Err / None;
  `None` is Instant-classified: at/after deadline ⇒ timeout; before ⇒ stream self-ended ⇒ success.
- **Snapshot timeout error** — `snapshot_timeout_error(instrument, md_type, context)`: code `timeout`,
  exit 6; message names instrument, deadline, md-type, likely causes, and the `--md-type delayed` hint.
- **Instrument label** — `<symbol>` (quote) / `<symbol> <expiry> <strike> <C|P>` (option-quote).
