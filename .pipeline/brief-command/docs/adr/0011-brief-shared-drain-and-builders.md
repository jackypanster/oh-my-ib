# ADR 0011 — brief: consolidated account_updates drain + shared row builders

Status: accepted (arch, 2026-07-03). Feature: brief-command. Companion to ADR 0010.

## Context

Three sections derive from the SAME gateway stream: `account_summary` (AccountValue keys),
`positions` (PortfolioValue rows), and `pnl_by_position`'s discovery (conid+symbol from
PortfolioValue). Today `account.rs`, `positions.rs`, `pnl_by_position.rs` each subscribe
`account_updates` independently — three commands, three subscriptions. `reqAccountUpdates` is a
singleton-style shared subscription ("Only one account can be subscribed at a time",
`accounts/sync.rs:232`), and its back-to-back re-subscribe across processes is the EAGAIN trigger.
Inside brief, re-subscribing it three times sequentially would be legal but re-runs the same
stream 3× and re-opens the async-release window brief exists to remove.

PRD criterion 2 (byte-shape-identical sections) forbids duplicated shaping logic: duplicate code
drifts; shared code cannot.

## Decision

1. **brief drains `account_updates` ONCE** and feeds all three consumers from that single pass:
   accumulate summary fields (AccountValue), position rows (PortfolioValue), and the
   `(conid, symbol)` discovery list (PortfolioValue) simultaneously, until `AccountUpdate::End`.
2. **Row/field shaping is extracted into shared builders** used by BOTH the existing commands and
   brief (reuse-by-refactor; public command fns keep their own connect + drain, behavior
   unchanged):
   - `account.rs`: summary accumulator seam — absorb `(key, value, currency)` AccountValue pairs;
     emit the 5-key summary object (minus `account`).
   - `positions.rs`: `position_row(&AccountPortfolioValue) -> Value` — the exact 9-key row.
   - `pnl_by_position.rs`: `sweep_pnl_singles(client, account, &[(conid, symbol)]) -> Vec<PnlSingleRow>`
     — the take-first sweep loop, parameterized by discovery input (its own command keeps its own
     drain; brief passes the consolidated drain's list). `shape_pnl_by_position` unchanged.
   - `pnl.rs` / `orders.rs` / `executions.rs`: `*_with_client(&Client, ...)` seams returning the
     section payload (object/array minus the `account` wrapper).
3. **`as_of` = `server_time()` formatted ISO-8601 UTC via `OffsetDateTime` inherent accessors**
   (`format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", t.year(), u8::from(t.month()), t.day(), ...)`).
   `decode_server_time` builds from `OffsetDateTime::from_unix_timestamp` → always UTC
   (`accounts/common/decoders/mod.rs:60-64`). No `time` crate dependency is added; no type from
   `time` is named (inherent methods + `u8: From<Month>` only). Rejected: adding `time` to
   `[dependencies]` for `Rfc3339` (new direct dep for one line); reusing `health`'s `{t:?}` Debug
   format (not ISO-8601, criterion 3).
4. **`assemble_brief` is the frozen pure seam**: `(account, as_of, six section Values) -> Value`
   with exactly the top-level key set `{account, as_of, account_summary, pnl, pnl_by_position,
   positions, orders, executions}`; pass-through, no re-shaping.

## Consequences

- One subscription instead of three inside brief; the singleton re-subscribe window never opens.
- Sections cannot drift from their source commands: same builder = same bytes. The six public
  commands are refactored but behavior-preserving (their own connect/drain/wrap paths intact;
  sibling frozen tests `executions_command.rs`/`pnl_by_position_command.rs`/`pnl_command.rs` pin
  the pure seams; `data_commands.rs`/`cli_contract.rs` pin the CLI contract).
- `orders` section keeps `omi orders` semantics verbatim — filtered by `--account` only when the
  operator passed it (NOT auto-filtered to the resolved account); single-account operation makes
  this moot, recorded to stop impl from "fixing" it.
- The summary accumulator takes the FIRST-seen currency alongside `NetLiquidation`
  (`account.rs:31-36` behavior) — preserved exactly.
