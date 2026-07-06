# PRD — live-gate-test-safety

Stage: prd · feature: live-gate-test-safety · repo: jackypanster/oh-my-ib · branch: main
Test-only safety fix (no product code). Author: cc. Blocks option-chain PR #25's merge gate.

## Problem

A test places REAL live orders. `tests/stk_orders_command.rs::live_buy_with_env_passes_gate_and_fails_
on_dead_gateway` (line 117) sets `OMI_ALLOW_LIVE=1` and runs the subprocess
`omi --format json buy AAPL 1 --limit 1 --live --host 127.0.0.1`, expecting error code `connection`.

Its INTENT: with the env set at the live port, the double write-gate does NOT reject, so execution
proceeds to connect; the author assumed `:4001` is DEAD (as in CI) so it fails with a connection error.

HAZARD: on a machine with the Tiger gateway UP on `127.0.0.1:4001`, the connect SUCCEEDS and the command
PLACES A REAL live buy order on account U20230856 (far $1 limit — contained, no fill, but a real resting
order). Confirmed live 2026-07-06: **3 such orders accumulated** from full-suite runs; cc cancelled all 3.
Consequence: the full `cargo test` merge gate is UNSAFE to run while the gateway is up — it is currently
BLOCKING option-chain PR #25.

## Goal

The stk-orders live-gate test NEVER places a real order regardless of local gateway state, while STILL
verifying the double-gate does not reject when `OMI_ALLOW_LIVE=1` at the live port. Full `cargo test`
becomes safe to run with the Tiger gateway UP.

## Decision (recommended; arch to confirm + ADR)

**Guard-skip on a reachable live gateway.** At the start of the one dangerous test, probe
`TcpStream::connect_timeout(127.0.0.1:4001, <short>)`:
- reachable (live gateway UP) ⇒ `eprintln!` a skip note and `return` — the test cannot run safely, so it
  is a no-op (NEVER places an order).
- not reachable (dead, e.g. CI) ⇒ assert `connection` error exactly as today (the gate-pass path is
  exercised; connect fails; no order is possible).

Rationale: bulletproof (an order is physically impossible in both branches), zero new deps, keeps the
subprocess env-isolation, one test touched, retains CI coverage. Std-only (`std::net::TcpStream`).

## Success criteria

1. Running `cargo test` (full suite) with the Tiger gateway UP on `:4001` places ZERO live orders
   (verify: `omi --live orders` empty before AND after the run). [operator live acceptance]
2. The dangerous test no longer connects to a live gateway when one is present (guard skips it). [read]
3. With no gateway on `:4001`, the test still asserts the `connection` error (gate-pass path covered).
   [offline / CI]
4. The four gate-REJECT tests (`live_buy/sell/cancel_without_env_is_config_error`,
   `hand_set_live_port_without_env_is_also_gated`) are UNCHANGED and still assert `config`. [read]
5. No product code changed — diff is `tests/stk_orders_command.rs` only. `require_live_write_gate` and
   all of `src/` untouched. [read]
6. `cargo build` + full `cargo test` (gateway down OR guarded) green; `cargo clippy --all-targets --
   -D warnings` clean.

## Scope

- IN: the ONE test `live_buy_with_env_passes_gate_and_fails_on_dead_gateway` in
  `tests/stk_orders_command.rs`. A small std-only guard helper may be added to that test file.
- OUT: any `src/` change; the gate logic `require_live_write_gate`; the four gate-REJECT tests; the
  paper dead-port test; any new crate dependency.

## Non-scope / rejected alternatives (for arch to record in the ADR)

- **Fast-fail loopback (`--host 127.0.0.2:4001`)**: keeps the test always-running but ASSUMES the
  gateway binds only `127.0.0.1`; if it binds `0.0.0.0` an order could still be placed. Rejected —
  binding-dependent, not bulletproof.
- **Lift env into `Config.allow_live` + pure in-process gate test**: cleaner unit test but touches
  `src/ib/trade.rs` (the write module) + `src/config.rs` + the gate's callers — too much blast radius
  for a test-safety fix, and mutates the safety-critical gate. Rejected for scope.
- **`#[ignore]` / delete the test**: drops the gate-pass coverage entirely. Rejected — guard-skip keeps
  CI coverage.

## Freeze / re-freeze note

This feature MODIFIES stk-orders' frozen spec file `tests/stk_orders_command.rs` (its original
spec-rev `3692c71`). The task stage RE-FREEZES that file under THIS feature (a new spec-rev). The frozen
red test must COMPILE and FAIL first: it should assert the NEW safe behavior (guard present / no live
connect when gateway up) in a way that fails against the current dangerous test. arch/task decide the
exact frozen assertion — note that "no order placed with gateway up" is an operator-live-acceptance
criterion, NOT a unit assertion (a test can't assert the negative safely). The frozen unit assertion is
likely the guard's presence/behavior on the dead-gateway path + the unchanged reject tests.

## Gotchas

- The dangerous test is the ONLY unsafe one; the four gate-REJECT tests are already safe (refused before
  connect; `omi()` does `env_remove("OMI_ALLOW_LIVE")`).
- The gate keys on `cfg.port == LIVE_PORT (4001)`; `--live` and `--port 4001` both trigger it.
- Do NOT run the full `cargo test` on this repo while the Tiger gateway is UP until this fix lands —
  the impl/review stages must use CARD-SCOPED tests only, or run with the gateway down.
- After this merges: option-chain PR #25 rebases onto it and its full-suite gate becomes safe.

## Verify

`cargo build` · `cargo test` (gateway down, or with the guard) · `cargo clippy --all-targets -- -D warnings`
· operator: `omi --live orders` empty before+after a full `cargo test` with the gateway UP.
