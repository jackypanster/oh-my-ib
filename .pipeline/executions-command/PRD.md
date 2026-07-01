# PRD — executions-command

Feature: a new read-only subcommand `omi executions` that reports the account's **current-day
executions (fills)** — the one missing piece of the order lifecycle — via TWS API `reqExecutions`.

## Problem

The agent-driven monitoring loop cannot answer "**did my order fill? at what price? what did I
trade today, and what commission did I pay?**" Code-first audit of the 8 shipped commands:

- `omi orders` (`src/ib/orders.rs`) emits only **working / open** orders (`all_open_orders`) — nothing
  that has already filled.
- `omi positions` (`src/ib/positions.rs`) carries per-position `realized_pnl`, but that is a **cumulative
  position-level** number, not an itemized fill log; it says nothing about *which* fills happened, when,
  at what price, or on symbols that netted flat / are no longer held.
- `omi pnl` / `omi account` are balances and P&L aggregates — no trade detail.

Across the whole order lifecycle the tool has "what's still working" (`orders`) but a **hole where
"what already filled today" belongs**. The agent (an LLM) **cannot derive** the fill log from existing
JSON — it would have to guess fills from position deltas, which is unreliable and misses flat/closed
names. This is a real capability hole, per the agent-first rule.

## Goal

Ship `omi executions`: connect → `client.executions(ExecutionFilter{account_code: <resolved>, ..default})`
→ drain the subscription → **join** each execution to its commission report by `exec_id` → disconnect →
emit JSON. Strictly **read-only** — no order path, no `OMI_ALLOW_LIVE` write gate. Reuses the existing
connect-per-command + drain-to-End pattern (`orders.rs` / `positions.rs`) and the `pnl_number` sentinel
seam.

## Success criteria (acceptance)

1. `omi executions` connects, requests the **current day's** executions for the resolved account
   (`ExecutionFilter.account_code` = resolved account; honors `--account` / first managed account like
   `account`/`pnl`), and prints one JSON object to stdout, exit 0:
   ```json
   { "account": "<id>", "executions": [
     { "exec_id": "<str>", "order_id": <int>, "perm_id": <int>, "time": "<IB server-time str>",
       "symbol": "<str>", "conid": <int>, "side": "BOT"|"SLD", "shares": <number>, "price": <number>,
       "cumulative_qty": <number>, "avg_price": <number>, "exchange": "<str>",
       "commission": <number|null>, "commission_currency": "<str>|null", "realized_pnl": <number|null> }
   ] }
   ```
2. **exec↔commission JOIN by `exec_id`.** Executions and commission reports arrive as two interleaved
   item kinds on the same subscription; the command accumulates both and merges commission fields onto
   each execution by `exec_id`. An execution with **no matching** commission report renders
   `commission` / `commission_currency` / `realized_pnl` as **`null`** (still a valid, complete row).
3. **`realized_pnl` reuses the `pnl_number` seam** (`src/ib/pnl.rs`): a finite non-sentinel value stays a
   number; IBKR's `Double.MAX_VALUE` (`1.7976931348623157e308`) / any non-finite / absent → JSON `null`.
   The agent must never see `1.7e308` reported as a dollar figure.
4. **Empty is success.** A day with no fills returns `{ "account": "<id>", "executions": [] }`, exit 0 —
   not an error. (An agent asking "what did I trade today?" on a flat day gets an honest empty list.)
5. **`side` is the canonical wire string** `"BOT"` / `"SLD"` (`ExecutionSide::as_str`), not Rust `Debug` —
   a stable agent-facing key.
6. `--format table` renders the same data human-readably (reuse the generic `src/output.rs` render).
7. Gateway-down / connect failure exits non-zero with the standard `{"error":{code,message,context}}`
   stderr envelope (same as every other command).
8. `cargo build`, `cargo test`, `cargo clippy --all-targets -- -D warnings` all green; all freeze gates empty.

## Scope

- New `src/ib/executions.rs`:
  - `pub fn executions(cfg: &Config) -> Result<serde_json::Value, AppError>` — the gateway seam:
    `super::connect` → `super::resolve_account` → build `ExecutionFilter { account_code: account.0.clone(),
    ..Default::default() }` → `client.executions(filter)` → drain `iter_data()`, mapping
    `Executions::ExecutionData` → an `ExecRow` (order preserved) and `Executions::CommissionReport` →
    a `CommissionRow`, then `merge_executions(rows, commissions)` → assemble `{account, executions}`.
  - `pub fn merge_executions(execs: Vec<ExecRow>, commissions: Vec<CommissionRow>) -> serde_json::Value`
    — the **pure, frozen-testable JOIN seam** (mirrors the `pnl_number` / `quote_price_tick` pure-seam
    convention): input order preserved; commission matched by `exec_id`; missing commission → `null`
    fields; `realized_pnl` run through `pnl_number`; a commission with an `exec_id` matching no execution
    is dropped (no phantom row). `ExecRow` / `CommissionRow` are **plain structs** (`String` / `f64` /
    `i32` / `i64` fields, `side` already a `String`) so the frozen test constructs them with **no `ibapi`
    import** — as `pnl_number` needs none.
- `src/ib/mod.rs`: `mod executions;` + `pub use executions::{executions, merge_executions};` (export the
  seam for the frozen test).
- `src/cli.rs`: add `Executions` to the `Command` enum (no args; doc-comment like the other read commands).
- `src/main.rs`: dispatch `Command::Executions => ib::executions(&config)`.

## Non-scope (explicitly NOT this card)

- **No filter flags.** `--symbol` / `--side` / `--days` / `--client-id` (the rest of `ExecutionFilter`)
  are a deliberate **future card** (`executions-filters`). Card 01 sets `ExecutionFilter` to default
  except `account_code`. (Operator-locked: minimal no-flag slice first.)
- **No `completed_orders`.** `client.completed_orders(api_only)` (`reqCompletedOrders`, order-level
  terminal states) is a **rejected** alternative — coarser (no per-fill price, no commission) and
  overlapping the existing `orders` command. Executions (fill-level) is the chosen surface.
- **No multi-day / historical fills.** `reqExecutions` returns **current-day only** by API design; the
  command makes no attempt to page prior days. "Yesterday's fills" is out of reach — query intraday.
- **No correction-collapsing.** IB signals a correction as an `exec_id` differing only after the final
  dot (`.01` → `.02`); v1 reports rows **as delivered** by the gateway, no merge/dedup of corrections.
- No order placement / modify / cancel; no write path; no `OMI_ALLOW_LIVE`; no new dependency.

## Decisions (locked)

- D1 **`executions` (fill-level) over `completed_orders` (order-level).** Fills carry price + commission +
  cumulative/avg price — the granular "what actually traded" answer; `completed_orders` overlaps `orders`.
- D2 **Minimal no-flag slice** (operator-locked). Server-side scoping via `ExecutionFilter.account_code`
  only; all other filter fields default. Smallest correct, independently shippable slice.
- D3 **exec↔commission JOIN in a pure frozen seam** `merge_executions`. Missing commission → `null`;
  `realized_pnl` reuses `pnl_number`. The gateway fn only extracts ibapi types → plain rows; all mergeable
  logic lives in the ibapi-free seam so the freeze covers it offline.
- D4 **`side` = canonical `"BOT"`/`"SLD"`** (`ExecutionSide::as_str`), not `Debug` — stable agent key
  (same rule as keeping machine-parseable output over Rust-formatted enums).
- D5 **`time` emitted raw** as ibapi delivers it (IB server-time string) — no reformatting. Deterministic,
  agent-first; `tz.rs` is connect-time alias registration, not output formatting.
- D6 **Empty result is success** (exit 0, `[]`) — not an error. A flat day is a valid answer.
- D7 **Read-only, `--live` allowed, no write gate.** Reuse `super::connect`, `resolve_account`,
  `output.rs` generic render, and the `pnl_number` null-helper. No new abstraction, no new dependency.

## Freeze coverage

Binary+lib crate: gateway behavior can't be exercised offline, so the freeze protects the **black-box CLI
contract** + the **pure JOIN seam** (the gateway wiring is reviewed-by-reading + live acceptance).

- Frozen (`tests/executions_command.rs`, offline, RED until impl):
  - **black-box** (assert_cmd, mirror `cli_contract.rs`): `omi --help` stdout contains `"executions"`;
    `omi executions --help` exits 0.
  - **pure seam** (NO `ibapi` import — `merge_executions` over plain `ExecRow`/`CommissionRow`):
    - matched join: one exec + its commission (same `exec_id`) → one row with numeric
      `commission` / `commission_currency` / `realized_pnl`.
    - unmatched exec (no commission) → `commission` / `commission_currency` / `realized_pnl` = `null`.
    - `realized_pnl` sentinel (`1.7976931348623157e308`) and `None` → `null` (via `pnl_number`).
    - input order preserved across ≥2 execs; `side` string passes through verbatim.
    - orphan commission (`exec_id` matching no exec) → dropped, no phantom row; empty input → `[]`.
- NOT frozen — reviewed-by-reading + operator live acceptance: `client.executions` wiring, the
  ibapi-type→row mapping, drain-to-End termination, account resolution, JSON assembly. Acceptance:
  `omi --live executions` after a day with fills → itemized fills with prices/commission; `[]` on a
  no-trade day; `--format table` readable.

## Verification

- Offline: `cargo build`, `cargo test` (incl. `cargo test --test executions_command`),
  `cargo clippy --all-targets -- -D warnings`.
- Live (operator, after the Tiger gateway reopens on `:4001`): on a day with trading activity,
  `omi --live executions` → fills present with `price` and (if the gateway sends them) `commission` /
  `realized_pnl`; on a flat day → `{ "executions": [] }`; `--format table` readable.

## Risks / fragile assumptions

- **Load-bearing:** the operator's gateway is **Tiger Brokers** (TWS-API-compatible, not real IBKR). Two
  independent degradations, both graceful:
  1. Tiger stubs `reqExecutions` → command returns `{ "executions": [] }` (still correct, not a crash).
  2. Tiger sends executions but **no** commission reports → `commission`/`commission_currency`/
     `realized_pnl` render `null` (rows still useful — price/side/qty present).
  Requires early `omi --live executions` acceptance on `:4001` to learn which regime Tiger is in.
- **Arch must resolve (the ADR-0007 analog):** does the `executions` subscription deliver
  `CommissionReport` items **before it terminates**, and **how does `iter_data()` terminate** for this
  request-id-scoped subscription? Contrast the two known shapes in this repo: `orders`/`positions` drain
  to an `End` marker; `pnl` is an unbounded stream with **no** `End` (take-first, ADR 0007). If commission
  reports can arrive after the execution stream's end, the drain strategy must account for it. Arch to
  confirm against the ibapi 3.1 source and record an ADR if the drain shape is non-obvious.
- `reqExecutions` is **current-day only** by API — this is a design limit, not a bug; document it so the
  agent does not promise historical fills.
- Correction rows (`exec_id` `.01`→`.02`) are reported as delivered; correction-collapsing is deferred
  (non-scope), so an agent summarizing fills should be aware duplicates-with-corrections can appear.
