# ADR 0005 — Paper default, live opt-in (safety model)

Status: accepted

## Context
The tool talks to a real brokerage account and is driven by an LLM. A misdirected command must not
accidentally hit the live account.

## Decision
- Default port **4002 (paper)**. `--live` selects **4001 (live)**.
- Read-only commands on `--live` are permitted (monitoring the real account is a valid use).
- Any **write** operation (future phases) additionally requires the env var `OMI_ALLOW_LIVE=1` — a
  second, deliberate gate beyond the flag.

## Rationale
- Defense in depth: the dangerous direction (writing to live) requires both an explicit flag and an
  explicit environment opt-in.
- Phase 1 is structurally read-only, so the flag alone is safe here; the env gate is reserved so the
  contract is already in place when trading lands.

## Consequences
- Phase 2 order code must check `--live` ⇒ require `OMI_ALLOW_LIVE=1` before transmit.
- Defaults bias every accidental invocation toward the paper account.
