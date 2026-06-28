# arch — connect-retry (follow-up B)

Small, additive change to the single connect path. No new module; both pieces live in `src/ib/mod.rs`.
No change to any command behavior beyond connect becoming resilient to transient errors.

## Pieces (both in `src/ib/mod.rs`)
1. `pub fn is_transient_io(kind: std::io::ErrorKind) -> bool` — pure classifier:
   `matches!(kind, WouldBlock | Interrupted | TimedOut)`. Public so the frozen test (std-only) can assert it.
2. `connect` retry loop:
   ```
   const MAX_CONNECT_RETRIES: u32 = 3;
   const BACKOFF_MS: u64 = 250;
   register tz aliases; loop {
     match Client::connect(addr, client_id) {
       Ok(c) => return Ok(c),
       Err(err) => {
         let transient = matches!(&err, ibapi::Error::Io(e) if is_transient_io(e.kind()));
         if transient && attempt < MAX_CONNECT_RETRIES { attempt+=1; sleep(BACKOFF_MS*attempt); continue }
         return Err(AppError::connection(format!("cannot connect to IB Gateway: {err}"), addr));
       }
     }
   }
   ```

## Why classify by io kind, not string
`ibapi::Error::Io(std::io::Error)` is a confirmed variant (errors.rs). EAGAIN → `ErrorKind::WouldBlock`,
refused → `ConnectionRefused`. Matching the kind is robust across OS/locale; string matching is not.

## Latency / safety
- Transient: ≤ 250+500+750 = 1.5s added worst case (only when retried).
- Permanent (refused = gateway down, the dead-port frozen test): `transient=false` ⇒ no retry ⇒ fails fast,
  so `tests/cli_contract.rs::connection_error_is_json_envelope` stays fast and green.

## Freeze coverage
Frozen (`tests/connect_retry.rs`, std-only): `is_transient_io` true for WouldBlock/Interrupted/TimedOut,
false for ConnectionRefused/NotFound. NOT frozen: the retry actually smoothing back-to-back
`account_updates` (gateway-dependent, intermittent) — reviewed + live back-to-back acceptance.

No ADR (small, additive, reversible; consistent with ADR 0003 stateless connect — retry is per-invocation).
