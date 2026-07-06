# ADR 0029 — live-gate test: guard-skip on a reachable gateway

Status: accepted · 2026-07-06 · feature: live-gate-test-safety

## Context

`tests/stk_orders_command.rs::live_buy_with_env_passes_gate_and_fails_on_dead_gateway` proves the live
double-gate PASSES with `OMI_ALLOW_LIVE=1` by running `buy AAPL 1 --limit 1 --live --host 127.0.0.1` and
expecting a `connection` error — it ASSUMES `:4001` is dead. With the Tiger gateway UP, the connect
succeeds and a REAL live buy order is placed (cc cancelled 3 such orders on 2026-07-06). The gate keys on
`cfg.port == LIVE_PORT (4001)`, so the test MUST target port 4001 to exercise the gate — but 4001 is
exactly where the real gateway listens. This is a test-safety defect, not a product bug.

## Decision

**Guard-skip on a reachable live gateway.** Add a std-only helper to the test file:

```rust
use std::net::TcpStream;
use std::time::Duration;

fn live_gateway_reachable() -> bool {
    // Probe the live port; a running gateway means the live-buy connect would SUCCEED and
    // place a REAL order, so the test cannot run safely here.
    "127.0.0.1:4001".parse().ok()
        .and_then(|addr| TcpStream::connect_timeout(&addr, Duration::from_millis(300)).ok())
        .is_some()
}
```

In the dangerous test, guard first:

```rust
if live_gateway_reachable() {
    eprintln!("skip live_buy_with_env_...: live gateway on :4001 — cannot safely exercise a live connect");
    return;
}
// gateway is dead ⇒ the gate-pass path is exercised and the connect fails with `connection`.
```

An order is now physically impossible: gateway UP ⇒ skip; gateway DOWN ⇒ connect fails before placement.

## Consequences

- Full `cargo test` is SAFE to run with the Tiger gateway UP (this test skips; no order placed).
- CI / gateway-down keeps the `connection`-error assertion (gate-pass coverage retained).
- On a dev machine with the gateway UP, the test is a logged no-op (green, not asserting) — acceptable:
  the safety-critical gate-REJECT direction stays fully tested by the four `..._without_env_is_config_
  error` / `hand_set_live_port_..._gated` subprocess tests (refused before connect — always safe).
- No product code changes. No new crate deps (std `TcpStream`/`Duration` only).

## Freeze coverage

NONE frozen. This is a test-only fix; the meaningful correctness ("no live order placed when a gateway is
up") CANNOT be a unit assertion (asserting it requires a live gateway — the exact hazard). Per CONTRACT
§Freeze coverage (the can't-freeze case): the test file `tests/stk_orders_command.rs` is this feature's
IMPL-PATH (coder-editable), `spec-paths` is EMPTY, and correctness is verified by (a) reviewed-by-reading
(guard present; 4 reject tests unchanged; no `src/` diff) and (b) operator live acceptance (`omi --live
orders` empty before AND after a full `cargo test` with the gateway UP).

## Alternatives rejected

- **Fast-fail loopback `--host 127.0.0.2:4001`**: assumes the gateway binds only `127.0.0.1`; a
  `0.0.0.0` bind would still place an order. Not bulletproof.
- **Lift env into `Config.allow_live` + pure in-process gate test**: touches `src/ib/trade.rs` (write
  module) + `src/config.rs` + the gate's callers — excessive blast radius for a test-safety fix.
- **`#[ignore]` / delete**: drops gate-pass coverage entirely (guard-skip keeps it in CI).
