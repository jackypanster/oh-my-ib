# ADR 0007 — Take the first reading of the unbounded PnL stream

Status: accepted

## Context
`omi pnl` requests account PnL via `ibapi` sync `client.pnl(&account, None) -> Subscription<PnL>`. Unlike
every other read command, this subscription is an **unbounded real-time stream with no terminator**:

- `account` drains `account_updates` until `AccountUpdate::End`.
- `quote` drains `market_data` until `TickTypes::SnapshotEnd`.
- `reqPnL` (this command) emits PnL ticks **continuously with no `End` marker** and keeps streaming
  until the client cancels.

Copying the sibling `for … in subscription.iter_data()` drain loop here would **block forever** after the
first reading — there is no marker to `break` on.

## Decision
Take **exactly one** PnL reading via `Subscription::next_data() -> Option<Result<PnL, Error>>`, then drop
the subscription (which cancels the stream and disconnects on scope exit). Do **not** iterate.

- `Some(Ok(pnl))` → build the JSON object.
- `Some(Err(e))` → `AppError::data` (stream error).
- `None` → `AppError::data` ("no PnL reading") — stream closed before any data.

## Rationale
- One reading is the whole contract: a snapshot of Daily/Unrealized/Realized PnL at request time
  (matches the stateless connect-per-command model, ADR 0003).
- `next_data()` is the crate's purpose-built "first data item" primitive (notices filtered) — clearer
  than `iter_data().next()` and avoids any accidental loop.
- Fail-fast on `None`/`Err` keeps the structured-error contract (ADR 0004) intact.

## Consequences
- Any **future** streaming-read command (e.g. the deferred `pnl-by-position` via `pnl_single`, or a
  `--watch` mode) must follow this take-first pattern, or explicitly opt into a bounded loop with its own
  stop condition + timeout. The drain-to-End pattern is **not** transferable to markerless streams.
- If a gateway ever fails to emit an initial PnL tick promptly, `next_data()` blocks (consistent with the
  sibling commands' blocking reads). If live acceptance reveals a hang, switch to
  `next_timeout(Duration)` — noted as the fallback, not the default, to match the no-explicit-timeout
  convention of the existing commands.
