# ADR 0003 — Stateless connect-per-command (no daemon)

Status: accepted

## Context
The tool is driven by an agent issuing one command at a time. Options: a long-lived daemon holding a
connection, or connect fresh each invocation.

## Decision
Each invocation does **connect → request → print → disconnect**. No daemon, no persistent connection.

## Rationale
- Robust and simple: no residual sockets, no stale-session bugs, no IPC.
- `ibapi` connect is fast enough for interactive use.
- A dead gateway surfaces immediately as a connection error rather than a silently stale daemon.

## Consequences
- Fixed default `client_id = 100`. Concurrent invocations must pass distinct `--client-id` (the agent
  drives sequentially, so collisions are unlikely).
- Slightly more per-command overhead vs a warm connection (acceptable for this workload).
