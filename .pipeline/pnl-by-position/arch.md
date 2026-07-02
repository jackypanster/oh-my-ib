# arch — pnl-by-position

Chosen shape for `omi pnl-by-position` (PRD D1–D6 locked). All claims verified against ibapi-3.1.0
crate source and this repo; citations inline. Binding stream decision: ADR 0009 (this feature) on top
of ADR 0007 (take-first, pre-committed by its Consequences).

**Correction vs the seq=1 journal handoff:** this crate is a **lib + bin split** (`oh_my_ib` lib +
`omi` bin, ADR 0006) — NOT "binary crate, no lib.rs". The frozen test imports the pure seam via
`use oh_my_ib::ib::{...}` exactly like `tests/executions_command.rs`. (Also: ADR 0007's real filename
is `0007-pnl-take-first-unbounded-stream.md`, not `...-reading.md` as the seq=1 handoff wrote.)

## Components (write-set for impl)

- **`src/ib/pnl_by_position.rs` (NEW)** — one module per command (repo convention; `pnl.rs` stays
  account-level). Three items:
  - `pub struct PnlSingleRow` — plain, **ibapi-free** (frozen test constructs these directly; mirror
    `ExecRow`): `conid: i32`, `symbol: String`, `position: f64`, `daily_pnl: f64`,
    `unrealized_pnl: f64`, `realized_pnl: f64`, `value: f64`.
  - `pub fn shape_pnl_by_position(rows: Vec<PnlSingleRow>) -> Value` — the **pure seam** (mirror
    `merge_executions`): emits `Value::Array`, one object per row, keys exactly
    `conid, symbol, position, daily_pnl, unrealized_pnl, realized_pnl, value`; order preserved;
    empty input → `[]`. Sentinel routing per ADR 0009: `daily_pnl`/`unrealized_pnl`/`realized_pnl`/
    `value` each through `pnl_number(Some(x))` (`src/ib/pnl.rs`); `conid`/`symbol`/`position` raw.
  - `pub fn pnl_by_position(cfg: &Config) -> Result<Value, AppError>` — the gateway fn (NOT frozen):
    1. `super::connect(cfg)` → `super::resolve_account(&client, cfg)` (existing seams, `src/ib/mod.rs`).
    2. **Discovery**: `client.account_updates(&account)`, drain to `AccountUpdate::End` collecting
       `(conid, symbol)` from every `PortfolioValue` (the `positions.rs` pattern; PRD D6 — qty==0 rows
       included, no dedupe, mirror `positions.rs`). Drop the subscription (Drop → `cancel()`,
       ibapi `subscriptions/sync.rs:284-289`).
    3. **Sweep**: per `(conid, symbol)` in discovery order:
       `client.pnl_single(&account, ContractId::from(conid), None)` (`ContractId(pub i32)` + `From<i32>`,
       ibapi `accounts/types.rs:71/86`) → `next_data()` take-first (ADR 0007/0009 — markerless stream,
       NEVER drain) → drop subscription → build `PnlSingleRow`. `position`/`value`/PnL fields come from
       the `PnLSingle` reading (fresher than the portfolio snapshot); `symbol` from discovery
       (`PnLSingle` carries no contract identity — verified `accounts/mod.rs:172`).
       **Fail-fast** (ADR 0009): `Some(Err)`/`None` from any read → `AppError::data` naming the
       failing conid; no partial output.
    4. `json!({"account": account.0, "by_position": shape_pnl_by_position(rows)})`.
- **`src/cli.rs`** — add flat variant `Command::PnlByPosition` (clap derive kebab-cases it to
  `pnl-by-position`; no args struct), doc comment `/// Per-position PnL (daily, unrealized, realized)`.
- **`src/main.rs`** — dispatch arm `Command::PnlByPosition => ib::pnl_by_position(&config)`.
- **`src/ib/mod.rs`** — `mod pnl_by_position;` +
  `pub use pnl_by_position::{pnl_by_position, shape_pnl_by_position, PnlSingleRow};`.
- **No changes** to `config.rs`, `output.rs`, `error.rs` — `--format table` renders the
  `{account, <array-of-objects>}` shape generically (same envelope `executions` already ships).

## Interleaving safety (verified, ADR 0009)

`account_updates` is a **shared-channel** subscription (routed by message type,
`request_helpers::blocking::shared_request(OutgoingMessages::RequestAccountData, ...)`,
ibapi `accounts/sync.rs:224-228`; "only one account can be subscribed at a time"). `pnl_single` is
**request-id-routed** (`PnLSingleRequest{request_id}` / `cancel_by_id!(CancelPnLSingle)`,
`accounts/common/encoders.rs:54-67`). Routing domains are disjoint and the phases are sequential
(discovery fully drained + dropped before the first `pnl_single`), so no interleaving hazard. The
known Tiger EAGAIN quirk is a **reconnect** (same client_id, back-to-back processes) issue handled at
`super::connect` (`src/ib/mod.rs` retry seam); it does not apply within one connected session.

## JSON contract (frozen keys)

```json
{"account":"<id>","by_position":[
  {"conid":265598,"symbol":"AAPL","position":100.0,
   "daily_pnl":52.3,"unrealized_pnl":1204.5,"realized_pnl":null,"value":21050.0}
]}
```

snake_case; PnL/value fields are `number | null` (null = IBKR unset sentinel / non-finite, PRD
criterion 2); flat account → `"by_position": []`, exit 0 (criterion 3); errors keep the
`{"error":{code,message,context}}` envelope + non-zero exit (criterion 4).

## Freeze coverage (for pipeline-task; per ADR 0006)

- **FROZEN** (`tests/pnl_by_position_command.rs`, offline, card-scoped runner):
  - Black-box CLI: `omi --help` lists `pnl-by-position`; `omi pnl-by-position --help` exits 0.
  - Pure seam via `use oh_my_ib::ib::{shape_pnl_by_position, PnlSingleRow}`: finite values pass
    through as numbers; sentinel `1.7976931348623157e308` → `null` in each of
    `daily_pnl`/`unrealized_pnl`/`realized_pnl`/`value`; non-finite (`NAN`/`INFINITY`) → `null`;
    `conid`/`symbol`/`position` unaltered; row order preserved; empty `Vec` → `[]`; exact key set.
- **NOT FROZEN** (reviewed-by-reading + operator live acceptance per PRD D3 merge gate): the
  `pnl_by_position(cfg)` gateway fn — discovery drain, sweep loop, take-first reads, fail-fast
  mapping, account resolution, JSON assembly, `--format table` rendering.

## Risks carried forward (impl/review must know)

- **PRD D3 merge gate**: PR merges ONLY after operator live-accepts `omi --live pnl` first, then
  `omi --live pnl-by-position` (Tiger reqPnL-family support unverified; gateway currently closed).
- Blocking `next_data()` × N positions: a single unresponsive conid hangs the command. Default stays
  blocking (sibling consistency); `next_timeout(Duration)` is the recorded fallback, applied only if
  live acceptance shows hangs (ADR 0009, mirroring ADR 0007's fallback discipline).
- Whether Tiger emits qty==0 `PortfolioValue` rows (closed-today realized PnL) is a live-acceptance
  observation, not a design blocker (PRD D6).
