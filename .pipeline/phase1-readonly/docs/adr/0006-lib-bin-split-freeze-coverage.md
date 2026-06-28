# ADR 0006 — lib + bin split; freeze coverage = black-box CLI contract

Status: accepted

## Context
The pipeline freeze gate can only protect a *testable* surface. The meaningful correctness of this tool
(do account/positions/quotes return the right values) requires a live IB Gateway and cannot be unit-
tested deterministically in CI.

## Decision
- Build a **library crate `oh_my_ib` + thin `omi` binary** so pure logic is unit-testable.
- The **frozen spec** (`spec-paths`) is the **black-box CLI contract** run via subprocess: subcommand
  presence, flag presence, exit codes, and the **connection-error envelope against a dead port**
  (deterministic, gateway-independent).
- The **live IB calls** (`ib/**`) are **not frozen** — reviewed by reading + covered by manual paper-
  account acceptance. Config-precedence and output-shaping logic get impl-authored `#[cfg(test)]` unit
  tests (impl-paths), not frozen.

## Rationale
- Honest freeze coverage: freeze what is deterministic offline; don't fake-freeze gateway-dependent code.
- The lib split lets the connection-error path and shaping be tested without a network.

## Consequences
- `pipeline-review` must READ `ib/**` for correctness (the freeze gate doesn't cover it) and confirm the
  operator's manual paper acceptance before merge.
- Card granularity is coarser than a fully-unit-testable service (2 cards, not 7).
