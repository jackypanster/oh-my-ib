# PRD — tz-aliases (Phase 1.1)

Feature: `omi` auto-registers gateway timezone aliases so it connects out-of-the-box to gateways that
report a non-IANA timezone abbreviation. Follow-up A from `.pipeline/phase1-readonly/reviews/review-05.md`.

## Problem
The operator runs a **Tiger Brokers gateway** (live, :4001) that reports timezone **"HKT"** during the
API handshake. rust-ibapi rejects unknown abbreviations and aborts the connection unless
`IBAPI_TIMEZONE_ALIASES=HKT=Asia/Hong_Kong` is exported first. Requiring that env var on every run
breaks the "daily driver that just works" goal (live acceptance proved `omi` is otherwise correct).

## Goal
Before connecting, `omi` registers a curated set of **unambiguous** timezone aliases (including
`HKT → Asia/Hong_Kong`) via `ibapi::register_timezone_alias`, so the handshake succeeds with no env var.
The `IBAPI_TIMEZONE_ALIASES` env (read natively by ibapi) still works and is additive.

## Success criteria
1. With **no** `IBAPI_TIMEZONE_ALIASES` set, `omi --live health` connects to the Tiger gateway (live acceptance).
2. The built-in alias table contains `HKT → Asia/Hong_Kong` (frozen offline test).
3. Setting `IBAPI_TIMEZONE_ALIASES` still works (not regressed).
4. `cargo build`, `cargo test`, `cargo clippy --all-targets -- -D warnings` green; freeze gate empty.

## Scope
- New lib module `src/tz.rs`:
  - `pub fn builtin_aliases() -> &'static [(&'static str, &'static str)]` — the curated map.
  - `pub fn register_builtin_aliases()` — registers each via `ibapi::register_timezone_alias`, once.
- Wire `register_builtin_aliases()` into `ib::connect` (before `Client::connect`), guarded by `std::sync::Once`.
- `pub mod tz;` in `lib.rs`.

## Non-scope
- No new subcommand, no config-file alias table (env var already covers user extension).
- Only **unambiguous** abbreviations (no CST/IST/etc. that map to multiple zones).
- No change to the Phase 1 read-only contract or any other command behavior.

## Decisions
- Curated built-in set (all unambiguous): `HKT→Asia/Hong_Kong`, `JST→Asia/Tokyo`, `KST→Asia/Seoul`,
  `SGT→Asia/Singapore`. HKT is the one actually hit; the others are safe regional neighbours.
- Register before connect, idempotent via `Once` (registering the same alias twice must not error/panic).
- Env var remains authoritative for anything the user adds.

## Freeze coverage
Frozen (`tests/tz_aliases.rs`, offline-deterministic): `tz::builtin_aliases()` contains
`HKT → Asia/Hong_Kong` and the table is non-empty with no empty entries. NOT frozen — reviewed +
**live acceptance**: that the gateway handshake actually succeeds without the env var (needs the live
gateway; the operator/me runs `unset IBAPI_TIMEZONE_ALIASES; omi --live health`).
