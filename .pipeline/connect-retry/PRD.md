# PRD — connect-retry (review-05 follow-up B)

Feature: `ib::connect` retries on transient connection errors so back-to-back account-scoped commands
stop intermittently failing with EAGAIN.

## Problem
Phase 1 live acceptance: running `omi --live account` then immediately `omi --live positions` (both open a
fresh `account_updates` / reqAccountUpdates connection with the same default client_id) intermittently
fails the second one with `Resource temporarily unavailable (os error 35)` (EAGAIN). A manual retry
succeeds — the gateway just hadn't released the prior subscription/client_id yet. For an agent driving
commands in sequence this is flaky and surprising.

## Goal
`ib::connect` transparently retries a **transient** connection failure (EAGAIN/WouldBlock and similar)
a few times with a short backoff, so sequential commands succeed without the operator retrying.
Non-transient failures (e.g. connection refused = gateway down) still fail fast — no pointless retries.

## Success criteria
1. `is_transient_io(WouldBlock)` is true; `is_transient_io(ConnectionRefused)` is false (frozen offline).
2. `omi health --port 65000` (dead port → refused) still fails fast with the connection envelope, exit 2
   (no multi-second retry storm) — existing frozen test stays green.
3. Live: `omi --live account` immediately followed by `omi --live positions` (same default client_id)
   succeeds repeatedly with no surfaced EAGAIN.
4. `cargo build`, `cargo test`, `cargo clippy --all-targets -- -D warnings` green; freeze gates empty.

## Scope
- `src/ib/mod.rs`:
  - `pub fn is_transient_io(kind: std::io::ErrorKind) -> bool` — true for `WouldBlock | Interrupted | TimedOut`.
  - `connect`: bounded retry loop — on `Err(ibapi::Error::Io(e))` with `is_transient_io(e.kind())`, retry
    up to `MAX_CONNECT_RETRIES` (3) with linear backoff (`250ms * attempt`); otherwise return the error.

## Non-scope
- No client_id rotation / randomization (keep ADR 0003's fixed default 100; retry addresses the cause).
- No connection reuse / daemon (stays stateless per ADR 0003).
- No new flag; backoff/retry count are internal constants.

## Decisions
- Classify by `std::io::ErrorKind` via `ibapi::Error::Io(std::io::Error)` (confirmed variant), NOT by
  string matching — robust across OS/locale. EAGAIN maps to `WouldBlock`.
- Retryable kinds: `WouldBlock`, `Interrupted`, `TimedOut`. Everything else (incl. `ConnectionRefused`)
  is permanent → fail fast.
- 3 retries, 250ms·attempt backoff (≤1.5s worst case added latency, only on transient).

## Freeze coverage
Frozen (`tests/connect_retry.rs`, offline, std-only — no ibapi dev-dep): `is_transient_io` classifies
WouldBlock/Interrupted/TimedOut as transient and ConnectionRefused/NotFound as not. NOT frozen — reviewed
+ live acceptance: the retry actually smooths back-to-back account_updates (gateway-dependent; EAGAIN is
intermittent, so acceptance = back-to-back success across several runs, not a forced failure).
