# ADR 0038 — Bound snapshot drains with a total deadline (`next_timeout(remaining)`)

Status: accepted
Amends: ADR 0013 (quote drain posture "deliberately NOT timeout-wrapped"), ADR 0019 D2 (option-quote
same posture). Sibling of ADR 0012 (take-first twin) / ADR 0016 (Instant-classified End drains).

## Context

`quote_one` (`src/ib/quote.rs:76`) and `option_quote` (`src/ib/option_quote.rs:215`) drain
`market_data(..).snapshot()` with bare `iter_data()` until `TickTypes::SnapshotEnd`. The premise
"SnapshotEnd always arrives" (read-timeouts PRD Non-scope) is live-disproven 2026-09-26 on IB Gateway
10.45 `:4001`: `omi --live --md-type live quote NVDA|SPY` receives no tick, no error, no SnapshotEnd,
and blocks until killed (120s / 40s). These are the only two snapshot call sites.

IB contract (📖 doc-cited, TWS API "Streaming Data Snapshots" + EWrapper reference): tickSnapshotEnd is
"Expected to occur 11 seconds after beginning of request". So a healthy snapshot's tail is ≈11s — a
10s bound (`TAKE_FIRST_TIMEOUT`) would false-timeout healthy snapshots.

## Decision

1. New shared pub const `SNAPSHOT_DEADLINE: Duration = 20s` in `src/ib/mod.rs` — ONE deadline for
   the WHOLE snapshot (measured from just after `subscribe()`), not a per-item window. Fixed, no
   flag/config (mirrors read-timeouts D3).
2. Replace each bare `for tick in subscription.iter_data()` with a total-deadline loop over
   `subscription.next_timeout(remaining)` where `remaining = deadline.saturating_duration_since(now)`:
   - `Some(Ok(SubscriptionItem::Data(TickTypes::SnapshotEnd)))` ⇒ break, success (unchanged).
   - `Some(Ok(SubscriptionItem::Data(tick)))` ⇒ existing per-tick handling (unchanged).
   - `Some(Ok(SubscriptionItem::Notice(_)))` ⇒ skip (continue) — parity with `iter_data`'s
     `FilterData`, which drops notices (warn-logs them); stream continues.
   - `Some(Err(e))` ⇒ existing `AppError::data("market_data stream: {e}", ctx)` (unchanged).
   - `None` AND `Instant::now() >= deadline` ⇒ `snapshot_timeout_error(..)` (code `timeout`, exit 6).
   - `None` before the deadline ⇒ stream self-ended ⇒ break with ticks gathered — IDENTICAL to the
     old `iter_data` loop ending on `None` (ADR 0016 Instant-classification precedent).
3. Pure builder `pub fn snapshot_timeout_error(instrument: &str, md_type: &str, context: &str) -> AppError`
   in `src/ib/mod.rs`, re-exported as `oh_my_ib::ib::snapshot_timeout_error`. Message (single line):
   `no SnapshotEnd within 20s for {instrument} (md-type={md_type}) — no real-time snapshot entitlement for this instrument, or market closed with no ticks; try --md-type delayed`
   (the `20` is formatted from `SNAPSHOT_DEADLINE.as_secs()`, never a literal).
   Contexts: `quote/<symbol>` (ADR 0013) / `option-quote`. Instrument labels: quote ⇒ `<symbol>`;
   option-quote ⇒ `<symbol> <expiry> <strike> <C|P>` (normalized right).
4. Batch `quote`: timeout of any symbol fails the whole command (ADR 0013 D3 fail-fast, PRD D1);
   the timed-out `Subscription` is dropped when `quote_one` returns ⇒ `Drop::cancel()` sends
   CancelMktData (📖 ibapi-3.1.0 sync.rs:78-99 skips cancel ONLY if `snapshot_ended`; :284-289).
5. `ErrorKind::Timeout` doc comment (`src/error.rs:10`) broadens from "gateway-side wedge; cure is
   restart" to "a bounded read produced no data in time (see the message for the cause/cure)".
   Code/exit unchanged.

## Rationale

- `timeout_iter_data(d)` is a PER-ITEM window (resets on every tick/notice): a trickle of ticks could
  extend a drain unboundedly, and the healthy ≈11s silent tail after the first ticks would need a
  window > 11s anyway — a total deadline states the real invariant ("≤ 20s per snapshot") directly.
- `next_timeout` already loops internally on skipped routed messages within its window (sync.rs:
  222-240), and `handle_response` is shared with `next()` ⇒ Data/Err/EndOfStream semantics are
  identical to `iter_data`; only Notice filtering and the None split are ours.
- Whole-command `timeout` reuses the machine contract agents already parse (read-timeouts D2).

## Consequences

- `omi quote` / `option-quote` can never block > ~20s per snapshot (+ connect).
- Worst-case batch latency unchanged for healthy symbols (≈11s each, sequential); a stuck symbol now
  costs 20s then fails the batch.
- Success output shape: zero change (N=1 byte-identity red line holds).
- Notices are no longer warn-logged on these two paths (the crate's FilterData logged them; we skip
  silently). Acceptable: `omi` does not surface crate logs to the agent.
