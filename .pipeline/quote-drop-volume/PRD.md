# PRD — quote-drop-volume (review-05 follow-up C)

Feature: `quote` emits only price ticks; the unreliable delayed-volume (and other size) ticks are dropped.

## Problem
`omi --live quote AAPL --md-type delayed` returns `DelayedVolume` like `13986886088824` (1.4e13), an
impossible share count. Root cause (code-first): `quote` passes every `TickTypes::Size(s).size` (f64)
straight through. The gateway's delayed-volume tick has no reliable/constant scaling — today's value
÷ `history`'s real same-day volume (11.3M) ≈ 1.24M, and the ratio is not constant across days. The
operator runs a **Tiger Brokers gateway** (not pure IBKR), so the volume-tick encoding is likely
gateway-specific. The price ticks (DelayedClose/High/Low/Open) are all correct — the defect is only in
the size/volume ticks. A misleading number is worse than no number for an agent-consumed JSON.

## Goal
`quote` outputs only the (reliable) **price** ticks. All `TickTypes::Size` ticks — which include
volume and bid/ask/last sizes — are dropped. Volume belongs to `history` (whose volume is correct).
No magic scaling is invented (it would be guessy and gateway-specific).

## Success criteria
1. `quote_price_tick(&TickTypes::Price(..))` returns `Some((label, price))`; for any non-price tick
   (Size/PriceSize/SnapshotEnd/…) it returns `None` (frozen, offline).
2. Live: `omi --live quote AAPL --md-type delayed` returns price keys (close/high/low/open/last as
   available) and **no** `*Volume` / size key. stdout is still valid JSON with `symbol`, `delayed`, `ticks`.
3. `cargo build`, `cargo test`, `cargo clippy --all-targets -- -D warnings` green; all freeze gates empty.

## Scope
- `src/ib/quote.rs`:
  - Add `pub fn quote_price_tick(tick: &TickTypes) -> Option<(String, f64)>` — `Some` only for
    `TickTypes::Price` (`(format!("{:?}", p.tick_type), p.price)`), `None` otherwise.
  - Rewrite the tick loop to break on `SnapshotEnd` and insert only `quote_price_tick(&tick)` results
    (the `TickTypes::Size` arm is removed).
- `src/ib/mod.rs`: `pub use quote::quote_price_tick;` so the frozen test can import it.

## Non-scope
- No tick-label normalization to a stable schema (keep the existing debug-name keys like `DelayedClose`).
- No PriceSize / bid-ask handling, no "last size", no scaling attempt, no new flag.
- No change to any other command. `history` already returns correct volume — unchanged.

## Decisions
- Drop **all** `Size` ticks (per operator: "price ticks only"), not just volume — sizes share the same
  unreliable Decimal/gateway encoding, and a snapshot quote's core is price.
- Expose `quote_price_tick` as the pure, frozen-testable seam (mirrors `is_transient_io` / `tz`).
- Volume stays available via `omi history` (verified correct).

## Freeze coverage
Frozen (`tests/quote_ticks.rs`, offline; needs `ibapi` as a dev-dependency to construct `TickTypes`):
a `TickTypes::Price` → `Some`, a `TickTypes::Size` (e.g. a volume tick) → `None`. NOT frozen — reviewed
+ live acceptance: `omi --live quote AAPL` shows price keys and no volume/size key.
