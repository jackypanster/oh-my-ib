# arch — pnl-command

Locked architecture for `omi pnl` (account-level Daily/Unrealized/Realized PnL). Grounded in code
(every PRD claim verified against the repo); reuses Phase 1 ADRs 0001–0006 unchanged, adds ADR 0007.

## Chosen shape

One new gateway-layer module `src/ib/pnl.rs` mirroring `src/ib/account.rs`, split into a
gateway-dependent function and a pure, frozen-testable seam — exactly the `quote.rs` /
`quote_price_tick` split.

```
src/ib/pnl.rs
  pub fn pnl(cfg: &Config) -> Result<serde_json::Value, AppError>   // gateway-dependent; NOT frozen
  pub fn pnl_number(raw: Option<f64>) -> serde_json::Value          // pure sentinel filter; FROZEN
```

### Component boundaries & wiring (verified)

- `src/ib/pnl.rs` (new) — the only file with real logic.
- `src/ib/mod.rs` — add `mod pnl;` + `pub use pnl::{pnl, pnl_number};`. (`connect` + `resolve_account`
  are already `pub(crate)` here and return `Client` / `AccountId` — exactly what `client.pnl` needs.)
- `src/cli.rs` — add `Pnl` to `enum Command` (no args; doc-comment like the other read commands).
- `src/main.rs` — one dispatch line: `Command::Pnl => ib::pnl(&config),`. `run()` already returns
  `serde_json::Value`; `main` already applies `--format` via `output::emit_success`.
- `src/output.rs` — **untouched**. `render_table` is fully generic over `serde_json::Value` (recursive
  key→value), so `--format table` works for `pnl` with zero card code (a `null` field renders as `null`).
  Table is explicitly *not* part of the frozen contract.

### Data flow

```
omi pnl
  → Config::load().merge_flags          (existing)
  → ib::pnl(cfg):
      client   = super::connect(cfg)                       (existing; retries transient EAGAIN)
      account  = super::resolve_account(&client, cfg)      (existing; --account / first managed)
      sub      = client.pnl(&account, None)  -> Subscription<PnL>     (ibapi 3.1.0 sync)
      reading  = sub.next_data()             -> Option<Result<PnL,Error>>   // TAKE EXACTLY ONE — see ADR 0007
      build JSON:
        { "account": account.0,
          "daily_pnl":      pnl_number(Some(reading.daily_pnl)),
          "unrealized_pnl": pnl_number(reading.unrealized_pnl),
          "realized_pnl":   pnl_number(reading.realized_pnl) }
  → output::emit_success(value, format)   (existing; json | generic table)
```

`ibapi::accounts::PnL { daily_pnl: f64, unrealized_pnl: Option<f64>, realized_pnl: Option<f64> }`
(verified at `accounts/mod.rs:160`). `daily_pnl` is non-Option but still passes through `pnl_number`
(defensive: it too can carry the unset sentinel).

## The sentinel filter (`pnl_number`) — the one piece of real logic

IBKR's TWS API encodes "no value / not applicable" as `Double.MAX_VALUE`, which is **exactly** Rust's
`f64::MAX` (`1.7976931348623157e308`) and round-trips through `ibapi`'s decode as a real `f64`. Operator
decision A (locked in PRD): such a value, and any non-finite value, renders as JSON `null`.

```
pnl_number(raw):
  match raw:
    Some(x) if x.is_finite() && x.abs() != f64::MAX  => Value::from(x)   // real number
    _                                                => Value::Null      // None | NaN | ±inf | sentinel
```

Pure, total, no I/O, no `ibapi` import (operates on plain `Option<f64>`) → trivially frozen offline.

## Frozen surface (freeze coverage — read by review)

Binary+lib crate; gateway behavior cannot run offline (ADR 0006). The freeze protects the **black-box
CLI contract** + the **pure `pnl_number` seam**:

- Frozen — `tests/pnl_command.rs` (new, offline, RED until impl):
  - black-box (extends `tests/cli_contract.rs` style, `assert_cmd`): `omi --help` stdout contains `pnl`;
    `omi pnl --help` exits 0.
  - pure seam (direct unit calls, **no ibapi import needed**):
    `pnl_number(Some(123.45)) == json!(123.45)`;
    `pnl_number(Some(1.7976931348623157e308)) == Value::Null`;   // == f64::MAX
    `pnl_number(Some(f64::INFINITY)) == Value::Null`;
    `pnl_number(Some(f64::NAN)) == Value::Null`;
    `pnl_number(None) == Value::Null`.
- NOT frozen — reviewed-by-reading + operator live acceptance (gateway reopens on `:4001`):
  `connect`/`resolve_account` reuse, `client.pnl()` wiring, `next_data()` take-first, JSON assembly,
  `--format table`. Acceptance: `omi --live pnl` → numeric `daily_pnl`, no `1.7e308` in any field.

## Decisions

- A1 **Mirror `account.rs` + `quote.rs` split.** Gateway fn (`pnl`) + pure seam (`pnl_number`). No new
  abstraction; reuses `connect`, `resolve_account`, `emit_success`, generic `render_table`.
- A2 **Take exactly one reading via `Subscription::next_data()`, never a drain-to-end loop.** `reqPnL` is
  an unbounded real-time stream with **no `End`/`SnapshotEnd` sentinel** (unlike `account_updates` →
  `AccountUpdate::End` and `market_data` → `TickTypes::SnapshotEnd`). The sibling `for … in iter_data()`
  loop would block forever. → **ADR 0007.** `None`/`Some(Err)` from `next_data()` ⇒ `AppError::data`.
- A3 **Sentinel/non-finite → null**, centralized in `pnl_number` (operator decision A; same spirit as
  `quote-drop-volume`: drop unreliable IB values rather than report garbage).
- A4 **No `currency` field.** `reqPnL` returns none; base currency already on `omi account`.
- A5 **Account-level only** (`model_code = None`); `--by-position` (`pnl_single`) is a deferred card.

## Reused / new ADRs

Reused unchanged: 0001 (TWS via ibapi), 0002 (sync client), 0003 (stateless connect-per-command),
0004 (JSON-first), 0005 (paper-default/live-opt-in; read-only on live allowed, no write gate),
0006 (lib/bin split → freeze coverage). New: **0007** (take-first read of an unbounded PnL stream).
