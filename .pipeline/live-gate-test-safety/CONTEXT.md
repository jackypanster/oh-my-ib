# CONTEXT — live-gate-test-safety

Glossary + conventions. Grounded in the codebase.

## Terms

- **Live double gate** — `require_live_write_gate(cfg)` (src/ib/trade.rs:175): a live write is refused
  unless `cfg.port == LIVE_PORT (4001)` AND `OMI_ALLOW_LIVE == "1"`. Offline-deterministic; runs BEFORE
  any connection. UNCHANGED by this feature.
- **Gate-REJECT test** — proves the gate REFUSES (env unset ⇒ `config` error, before connect). SAFE: no
  connection, no order. The four such tests stay as-is.
- **Gate-PASS test** — proves the gate does NOT refuse (env set ⇒ proceeds to connect). The ONLY
  dangerous one: if it reaches a live gateway it places a real order. This feature guards it.
- **Guard-skip** — a std-only probe (`TcpStream::connect_timeout` to `127.0.0.1:4001`) at the top of the
  gate-PASS test: reachable ⇒ skip (no-op, logged); unreachable ⇒ assert the `connection` error. Makes a
  live order physically impossible.
- **`omi()` test helper** — `Command::cargo_bin("omi")` with `env_remove("OMI_ALLOW_LIVE")` — every test
  starts from a clean env; the gate-PASS test re-adds the env via `cmd.env(...)`.

## Conventions (from the repo)

- Write calls live ONLY in `src/ib/trade.rs`; this feature touches NO `src/`.
- Some correctness can't be frozen as a unit test (CONTRACT §Freeze coverage); safety here is proven by
  reviewed-by-reading + operator live acceptance, not a frozen assertion.
- No new crate dependencies — std only.
- Tiger `:4001` behavior is never asserted in a test; it is operator live acceptance.
