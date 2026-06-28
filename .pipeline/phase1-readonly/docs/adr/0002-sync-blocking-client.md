# ADR 0002 — Use the sync (blocking) `ibapi` client, not async

Status: accepted

## Context
`ibapi` ships both an async (Tokio, default) and a sync (blocking) client. `omi` is a short-lived CLI
that issues one request per invocation.

## Decision
Use the **sync/blocking** client: `ibapi = { version = "3.1", default-features = false, features = ["sync"] }`,
type `ibapi::client::blocking::Client`.

## Rationale
- Process is short-lived, requests are sequential, there is no concurrency requirement.
- Blocking is the simplest correct shape; avoids pulling in and reasoning about a Tokio runtime.

## Consequences
- Straight-line code, easier error handling.
- If a future phase needs streaming/concurrent subscriptions, revisit (enable the async feature too).
