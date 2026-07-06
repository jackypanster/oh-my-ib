# arch — live-gate-test-safety

Stage: arch · decision of record: ADR 0029. Test-only fix; no `src/` change.

## Chosen shape

Guard the one dangerous test with a std-only live-gateway reachability probe; skip (no-op) when a
gateway is up, else keep the existing `connection`-error assertion. See ADR 0029 for the exact helper.

## Write-set for impl (all in `tests/stk_orders_command.rs`)

- ADD helper `live_gateway_reachable() -> bool` (std `TcpStream::connect_timeout` to `127.0.0.1:4001`,
  300ms) + `use std::net::TcpStream; use std::time::Duration;`.
- EDIT `live_buy_with_env_passes_gate_and_fails_on_dead_gateway`: guard at the top
  (`if live_gateway_reachable() { eprintln!(...); return; }`), then the UNCHANGED assertion body.
- DO NOT touch: the four gate-REJECT tests (`live_buy/sell/cancel_without_env_is_config_error`,
  `hand_set_live_port_without_env_is_also_gated`), the paper dead-port test, the pure-seam tests, the
  `omi()` / `expect_error_code` helpers.
- DO NOT touch any `src/` file. `require_live_write_gate` is unchanged.

## Component boundaries / pipeline handling

- spec-paths: **EMPTY** (nothing frozen — the meaningful test can't be frozen; ADR 0029 §Freeze coverage).
- impl-paths: `tests/stk_orders_command.rs` (this feature's coder-editable file; it WAS frozen under the
  DONE stk-orders feature `3692c71`, but under THIS feature it is an impl-path — the freeze gate only
  checks THIS feature's spec-paths, which are empty).
- No freeze commit (nothing to freeze). task writes only the card + metadata; impl edits the test file.
- verify (card-scoped): `cargo test --test stk_orders_command`. Note: this suite INCLUDES the guarded
  test; with the gateway UP it now SKIPS (safe); with it down it asserts `connection`. Either way green.
- full-verify (current.json): `[cargo build, cargo test]` — after this lands, the WHOLE suite is safe
  with the gateway up.

## Verification (the safety proof is operational, not a unit test)

- Reviewed-by-reading (codex): guard present + correct; 4 reject tests unchanged; diff is
  `tests/stk_orders_command.rs` ONLY (no `src/`).
- Operator live acceptance (cc at merge, gateway UP): `omi --live orders` empty → run full `cargo test`
  → `omi --live orders` STILL empty (zero orders placed). This is the criterion that proves the fix.

## Non-goals

Refactoring the gate; touching `src/`; the reject tests; new deps; asserting the negative in a unit test.
