# ADR 0004 — JSON-first output with a structured error envelope

Status: accepted

## Context
The primary consumer is an LLM agent parsing output; a human occasionally reads it. Errors must be
unambiguous and machine-detectable.

## Decision
- Default `--format json`: one JSON value to **stdout** on success.
- `--format table`: human-readable, same data.
- On failure: a structured envelope `{"error":{"code","message","context"}}` to **stderr** plus a
  non-zero **exit code** (per-variant, see arch.md). stdout stays empty on error.

## Rationale
- Deterministic parsing for the agent; exit code + stderr separation makes success/failure trivial to
  branch on in shell.
- Stable top-level keys per command decouple the agent from `ibapi` internals.

## Consequences
- Output shaping (`model` → JSON) is a defined, reviewable layer.
- Adding a command means defining its stable JSON shape, not just dumping library structs.
