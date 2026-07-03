# ADR 0012 — Bound take-first reads with the crate's timeout twin (`timeout_iter_data`)

Status: accepted

## Context

ADR 0007 chose blocking `next_data()` for take-first reads on the markerless `reqPnL` /
`reqPnLSingle` streams, and recorded `next_timeout(Duration)` as the fallback "if live acceptance
reveals a hang". That trigger FIRED on 2026-07-03 (live acceptance of brief-command, gateway
build 2026-06-25 on the operator's M1 install): `omi --live pnl` blocked forever on a wedged PnL
channel; killed processes orphaned their subscriptions and polluted the channel for all clients
until a gateway restart (`.pipeline/brief-command/reviews/review-01.md` §Live acceptance).

The exposure is exactly the two take-first seams (repo-wide grep, 2026-07-03): `pnl_with_client`
(`src/ib/pnl.rs:28`) and `sweep_pnl_singles` (`src/ib/pnl_by_position.rs:76`), serving four call
paths (`omi pnl`, `omi pnl-by-position`, and both inside `omi brief`). Every other gateway read
drains to an End marker (different failure class, never observed wedged — PRD non-scope).

## Decision

Swap `subscription.next_data()` for `subscription.timeout_iter_data(TAKE_FIRST_TIMEOUT).next()`
in both seams, where `TAKE_FIRST_TIMEOUT` is ONE shared pub const (10s) in `src/ib/mod.rs`.
Map the read's `None` to a new `AppError::timeout` (code `"timeout"`, exit 6) whose message
carries the wait and the cure ("gateway PnL channel may be wedged; restart the gateway"); the
sweep's arm keeps its fail-fast conid attribution. `Some(Ok)` / `Some(Err)` arms unchanged.

## Rationale

- `next_data()` ≡ `iter_data().next()` (ibapi-3.1.0 subscriptions/sync.rs:242-244);
  `timeout_iter_data(d).next()` (sync.rs:279-281) is its exact data-only timeout twin — same
  `FilterData` notice filtering, same `Option<Result<T, Error>>` shape. A one-expression swap
  preserves the healthy-path behavior byte-for-byte (PRD criterion 6).
- Rejected: raw `next_timeout(Duration)` (ADR 0007's literal fallback wording) — it yields
  `SubscriptionItem<T>` (Data OR Notice, sync.rs:222-237), forcing the seams to re-implement
  notice filtering + deadline bookkeeping the crate already provides.
- Rejected: reusing `AppError::data` for the timeout — a consuming agent must
  machine-distinguish "restart the gateway" from gateway data errors without message sniffing
  (operator decision, PRD D2).
- Rejected: configurable timeout — the wedge emits nothing forever, so tunability buys nothing;
  healthy first tick arrives <1s live (operator decision, PRD D3).

## Consequences

- No `omi` invocation can block indefinitely on the PnL channel; worst case for `brief` is
  (1 + N positions) × 10s serial. No more killed-process subscription litter from wedged reads.
- **None-collapse**: `None` means "timed out" OR "stream ended before data" (`stream_ended`
  short-circuits `None` instantly, sync.rs:223-225). The old `data`/"no PnL reading" arm folds
  into the `timeout` error; the closed-stream corner exits instantly (not after 10s) but now
  reports code `timeout`/exit 6 instead of `data`/4. Accepted: the two cases are
  indistinguishable at this layer and the cure guidance is identical.
- **Per-item window**: `timeout_iter_data` restarts the window per yielded item, so a stream
  emitting only filtered notices could extend the total wait beyond 10s. Accepted: extensions
  require actual traffic; the live-proven wedge is silent ⇒ 10s sharp.
- The wedge itself (gateway-state hazard) is NOT cured — bounded and named, cure stays an
  operator gateway restart. If a drain-to-End wedge is ever observed live, bounding those loops
  is a NEW feature with its own evidence (this ADR does not license blanket timeouts).
