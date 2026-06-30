# PRD — pnl-command

Feature: a new read-only subcommand `omi pnl` that reports **account-level** Daily / Unrealized /
Realized PnL, filling the one data gap in the monitoring story.

## Problem

The agent-driven monitoring loop ("how am I doing today?") cannot be answered today. Code-first audit:

- `omi account` (`src/ib/account.rs`) emits only static balances — `NetLiquidation`, `TotalCashValue`,
  `BuyingPower`, `AvailableFunds` — and **no PnL**.
- `omi positions` carries per-position *unrealized* PnL, but there is no **account-level Daily PnL**
  (today's change) anywhere, and the agent (an LLM) **cannot derive it** — Daily PnL needs the day's
  starting NAV / realized fills, which no existing command surfaces.

Daily PnL is the single highest-value monitoring datum and is currently unreachable. Per the
agent-first rule, the gap is a real capability hole, not something the LLM can compute from existing JSON.

## Goal

Ship `omi pnl`: connect → request account-level PnL via `ibapi` 3.1.0 sync `client.pnl(&account, None)`
→ take the first complete reading → disconnect → emit JSON. Strictly **read-only** — no order path, no
`OMI_ALLOW_LIVE` write gate. Mirrors the existing connect-per-command, subscription-take-first pattern.

## Success criteria

1. `omi pnl` connects, requests account PnL for the resolved account (honors `--account` / first managed
   account, like `account`), and prints one JSON object to stdout, exit 0:
   ```json
   { "account": "<id>", "daily_pnl": <number|null>, "unrealized_pnl": <number|null>, "realized_pnl": <number|null> }
   ```
2. **Sentinel/absent → `null`** (operator decision A): IBKR's TWS API uses `Double.MAX_VALUE`
   (`1.7976931348623157e308`) as its "no value / not applicable" marker, which leaks through `ibapi` as a
   real `f64`. Every PnL field is run through a sentinel filter: a non-finite value, or the IB sentinel,
   or a `None` (`unrealized_pnl`/`realized_pnl` are `Option<f64>`) renders as JSON `null`; a finite real
   value renders as a JSON number. The agent must never see `1.7e308` reported as a dollar P&L.
3. `--format table` renders the same data human-readably (reuse `src/output.rs`).
4. Gateway-down / no-reading-before-timeout exits non-zero with the standard `{"error":{code,message,context}}`
   stderr envelope (same as every other command).
5. `cargo build`, `cargo test`, `cargo clippy --all-targets -- -D warnings` all green; all freeze gates empty.

## Scope

- New `src/ib/pnl.rs`:
  - `pub fn pnl(cfg: &Config) -> Result<serde_json::Value, AppError>` — connect (`super::connect`),
    `resolve_account`, `client.pnl(&account, None)`, take the **first** `PnL` from the subscription, build
    the JSON object, return. (`ibapi::accounts::PnL { daily_pnl: f64, unrealized_pnl: Option<f64>,
    realized_pnl: Option<f64> }`.)
  - `pub fn pnl_number(raw: Option<f64>) -> serde_json::Value` — the pure, frozen-testable sentinel
    filter: `Some(finite, non-sentinel)` → `Value::from(f)`; `Some(sentinel | non-finite)` | `None` →
    `Value::Null`. (Mirrors the `quote_price_tick` pure-seam pattern.)
- `src/ib/mod.rs`: `mod pnl;` + `pub use pnl::{pnl, pnl_number};` (export the seam for the frozen test).
- `src/cli.rs`: add `Pnl` to the `Command` enum (no args; doc-comment like the other read commands).
- `src/main.rs`: dispatch `Command::Pnl => ib::pnl(&config)`.

## Non-scope (explicitly NOT this card)

- **No `--by-position`** — per-position PnL via `client.pnl_single(account, conId, None)` is a deliberate
  **future second card** (`pnl-by-position`); it needs conId resolution per position and is its own surface.
- No realized-fills / executions command (`client.executions(..)` — separate future feature).
- No `currency` field — `reqPnL` does not return one; the account base currency is already on `omi account`.
- No order placement / modify / cancel; no write path; no `OMI_ALLOW_LIVE`. No new dependency.
- No `snapshot` composite command (separate idea).

## Decisions (locked)

- D1 **Account-level only.** `client.pnl(&account, None)` (`model_code = None`). Highest-value, lowest-risk
  slice; per-position deferred.
- D2 **Sentinel → null (option A).** Clean, machine-parseable output is the agent-first contract; a
  disguised `1.7e308` is worse than an honest `null`. Same lesson as `quote-drop-volume` (drop unreliable
  IB values rather than report garbage). Centralized in the pure `pnl_number` seam.
- D3 **Take first reading, then disconnect.** `reqPnL` is a *continuous* real-time stream with **no `End`
  marker** (unlike `account_updates`). The command takes the first `PnL` item from `subscription.iter()`
  then drops the subscription (disconnect) — it must NOT wait for an `End` that never arrives.
- D4 **Read-only, no write gate.** `--live` permitted (read-only on live is allowed); no `OMI_ALLOW_LIVE`.
  Lowest review/safety bar → fastest through the pipeline.
- D5 **Reuse existing seams** — `super::connect`, `resolve_account`, `output.rs` table render, the JSON
  null-helper convention from `account.rs`. No new abstraction.

## Freeze coverage

This is a binary+lib crate; the gateway behavior cannot be exercised offline, so the freeze protects the
**black-box CLI contract** + the **pure sentinel seam**:

- Frozen (`tests/pnl_command.rs`, offline, RED until impl):
  - black-box: `omi --help` lists `pnl`; `omi pnl --help` exits 0 (extends the `cli_contract.rs` style).
  - pure seam: `pnl_number(Some(123.45))` → `123.45`; `pnl_number(Some(1.7976931348623157e308))` → `null`;
    `pnl_number(Some(f64::INFINITY))` → `null`; `pnl_number(None)` → `null`.
- NOT frozen — reviewed-by-reading + operator live acceptance once the Tiger gateway reopens on `:4001`:
  the `client.pnl()` wiring, account resolution, take-first-then-disconnect, JSON assembly. Acceptance:
  `omi --live pnl` returns a real numeric `daily_pnl` (and `null` or numbers for unrealized/realized).

## Verification

- Offline: `cargo build`, `cargo test` (incl. `cargo test --test pnl_command`), `cargo clippy --all-targets -- -D warnings`.
- Live (operator, after gateway reopens on `:4001`): `omi --live pnl` → numeric `daily_pnl`, no `1.7e308`
  in any field; `--format table` readable.

## Risks / fragile assumptions

- **Load-bearing:** the operator's #1 need is monitoring insight (Daily PnL), not trade automation. If that
  flips, Phase 2 (gated orders) is wanted instead — but that is the highest-risk surface, not this one.
- `reqPnL` is gateway-served; the operator runs a **Tiger Brokers** gateway (TWS-API-compatible), so the
  exact sentinel/encoding for "no value" is verified at live acceptance — the `pnl_number` filter is the
  defense (drops any non-finite / sentinel), independent of the precise gateway encoding.
- Live acceptance is **blocked until the operator reopens the gateway on `:4001`**; all offline gates
  (build/clippy/test + frozen red test) run without it, so build-to-green is not blocked.
