# ADR 0037 — audit write failure is fail-open (warn, never block the command)

## Status
Accepted (arch, agent-help-logs). Flagged to the operator: this deliberately deviates
from the global fast-fail preference, for the reasons below.

## Decision

If appending the audit line fails (unwritable dir, full disk, permission), omi prints
ONE plain-text line to stderr — `warn: audit log write failed: <err> (<path>)` — and the
command's own stdout, exit code, and JSON envelope behavior are completely unchanged.

The warn line is deliberately NOT the JSON error envelope: the envelope is the frozen
failure contract agents branch on (stderr envelope + non-zero exit). A warning
accompanying a SUCCESSFUL command must not be parseable as a command failure.

## Why fail-open and not fast-fail

- The audit log is observability, not the product. Fast-fail would let a full disk or a
  chmod mistake BLOCK trading commands — the launchd sma-monthly automation would then
  skip its monthly reconcile because logging (not trading) broke. Missing one audit line
  is strictly cheaper than missing one position reconcile.
- This is explicit, contextual handling — not exception swallowing: the failure is
  reported (stderr), attributable (path + cause), and bounded (one line).

## Revisit trigger

If live-mode write volume grows beyond the occasional gated order, reconsider:
fail-FAST for write commands only (buy/sell/cancel/option-*/ticks) while reads stay
fail-open — an audited trade trail may then be worth blocking on.
