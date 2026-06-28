# ADR 0001 — Use the TWS API via the `ibapi` crate (not the Web API)

Status: accepted

## Context
IBKR offers TWS API (socket) and Web API (REST/WebSocket, OAuth). Both require a local gateway process.
We need a Rust path.

## Decision
Use the **TWS API** through the native Rust crate **`ibapi`** (v3.1, MIT, actively maintained), connecting
to a local **IB Gateway**.

## Rationale
- Native Rust; the crate already encapsulates the binary socket protocol (no hand-rolled wire format).
- Most complete feature set: account, real-time + historical data, contracts, scanners.
- Web API's "lighter REST" advantage is undercut: it still needs the Client Portal Gateway, has a
  flakier session model, and fewer order features.

## Consequences
- Requires IB Gateway running + logged in (daily login/2FA; IBC automation deferred).
- No API keys (auth via the gateway). Ties us to `ibapi`'s release cadence and API surface.
