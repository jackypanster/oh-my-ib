# arch — executions-command

Locked architecture for `omi executions` (account current-day fills). Grounded in code — every PRD
claim verified against the repo AND the ibapi 3.1.0 source. Reuses Phase 1 ADRs 0001–0006 unchanged;
reuses the `pnl_number` seam from pnl-command; adds **ADR 0008** (drain-to-End + best-effort commission
join). The `reqExecutions` stream shape was the one real design risk — resolved below against the crate.

## Chosen shape

One new gateway-layer module `src/ib/executions.rs`, split into a gateway-dependent function and a
pure, frozen-testable JOIN seam — the same `quote.rs`/`quote_price_tick` and `pnl.rs`/`pnl_number` split.

```
src/ib/executions.rs
  pub fn executions(cfg: &Config) -> Result<serde_json::Value, AppError>              // gateway; NOT frozen
  pub fn merge_executions(execs: Vec<ExecRow>, comms: Vec<CommissionRow>) -> Value    // pure JOIN; FROZEN
  pub struct ExecRow { exec_id, order_id, perm_id, time, symbol, conid, side,         // plain, ibapi-free
                       shares, price, cumulative_qty, avg_price, exchange }
  pub struct CommissionRow { exec_id, commission, currency, realized_pnl: Option<f64> } // plain, ibapi-free
```

`ExecRow`/`CommissionRow` are plain structs (`String`/`f64`/`i32`/`i64`/`Option<f64>`, `side` already a
`String`) so `merge_executions` is a pure total function the frozen test drives with **no `ibapi` import**
— exactly what `pnl_number` bought us. The gateway fn does the only ibapi-typed work: extract each
`Executions` item into a row.

### Component boundaries & wiring (verified)

- `src/ib/executions.rs` (new) — the only file with real logic.
- `src/ib/mod.rs` — add `mod executions;` + `pub use executions::{executions, merge_executions};`.
  (`connect` + `resolve_account` are already `pub(crate)` here → `Client` / `AccountId`; `pnl_number` is
  already re-exported → `merge_executions` calls `super::pnl_number`.)
- `src/cli.rs` — add `Executions` to `enum Command` (no args; doc-comment like the other read commands).
- `src/main.rs` — one dispatch line: `Command::Executions => ib::executions(&config),`. `run()` already
  returns `serde_json::Value`; `main` already applies `--format` via `output::emit_success`.
- `src/output.rs` — **untouched**. `render_table` is fully generic + recursive over `Value`
  (`{key:[objects]}` already works for `positions`/`orders`), so `--format table` is free; a `null`
  commission field renders as `null`. Table is explicitly *not* part of the frozen contract.

### ibapi surface (verified against ibapi-3.1.0 source)

- `client.executions(filter: ExecutionFilter) -> Result<Subscription<Executions>, Error>`
  (`src/orders/sync.rs:144`) — read-only; docs: "Requests current day's (since midnight) executions".
- `ExecutionFilter { account_code: String, .. }` derives `Default` (`src/orders/mod.rs:1683`) →
  `ExecutionFilter { account_code: account.0.clone(), ..Default::default() }` (all other filters unset).
- `enum Executions { ExecutionData(ExecutionData), CommissionReport(CommissionReport) }`.
- `ExecutionData { request_id: i32, contract: Contract, execution: Execution }`. From it:
  `contract.symbol` → `symbol`, `contract.contract_id` → `conid` (same as `orders.rs`/`positions.rs`).
- `Execution { order_id: i32, execution_id: String, time: String, exchange: String, side: ExecutionSide,
  shares: f64, price: f64, perm_id: i64, cumulative_quantity: f64, average_price: f64, .. }`.
  `ExecutionSide::as_str() -> "BOT" | "SLD"` (`src/orders/mod.rs:1652`) → `side` (wire string, not Debug).
- `CommissionReport { execution_id: String, commission: f64, currency: String, realized_pnl: Option<f64>,
  .. }` → `CommissionRow`; `realized_pnl` fed through `pnl_number`.

### Data flow

```
omi executions
  → Config::load().merge_flags                         (existing)
  → ib::executions(cfg):
      client  = super::connect(cfg)                    (existing; retries transient EAGAIN)
      account = super::resolve_account(&client, cfg)   (existing; --account / first managed)
      filter  = ExecutionFilter { account_code: account.0.clone(), ..Default::default() }
      sub     = client.executions(filter) -> Subscription<Executions>
      execs: Vec<ExecRow> = []; comms: Vec<CommissionRow> = [];
      for item in sub.iter_data():        // drains to ExecutionDataEnd → EndOfStream → None (ADR 0008)
        match item? {
          Executions::ExecutionData(d)    => execs.push(ExecRow  from d.contract + d.execution),
          Executions::CommissionReport(c) => comms.push(CommissionRow from c),
        }
      rows = merge_executions(execs, comms)             // pure JOIN by exec_id
      { "account": account.0, "executions": rows }
  → output::emit_success(value, format)                (existing; json | generic table)
```

## The JOIN seam (`merge_executions`) — the one piece of real logic

```
merge_executions(execs, comms):
  index = HashMap<exec_id -> &CommissionRow>   from comms   (last write wins on dup exec_id)
  out = []
  for e in execs (ORDER PRESERVED):
    obj = { exec_id, order_id, perm_id, time, symbol, conid, side, shares, price,
            cumulative_qty, avg_price, exchange }
    match index.get(e.exec_id):
      Some(c) => obj += { commission: Value::from(c.commission),
                          commission_currency: Value::from(c.currency),
                          realized_pnl: pnl_number(c.realized_pnl) }   // reuse pnl.rs seam
      None    => obj += { commission: null, commission_currency: null, realized_pnl: null }
    out.push(obj)
  Value::Array(out)     // orphan commissions (exec_id matches no execution) are never emitted → no phantom row
```

Pure, total, no I/O, no `ibapi` import → trivially frozen offline. `realized_pnl` inherits the
`pnl_number` sentinel guarantee (IB `Double.MAX_VALUE` / non-finite / `None` → `null`), so a commission's
unset realized P&L never leaks `1.7e308`.

## Frozen surface (freeze coverage — read by review)

Binary+lib crate; gateway behavior cannot run offline (ADR 0006). The freeze protects the **black-box
CLI contract** + the **pure `merge_executions` seam** (gateway wiring is reviewed-by-reading + live):

- Frozen — `tests/executions_command.rs` (new, offline, RED until impl):
  - black-box (extends `tests/cli_contract.rs` style, `assert_cmd`): `omi --help` stdout contains
    `executions`; `omi executions --help` exits 0.
  - pure seam (direct unit calls on `merge_executions`, **no ibapi import**):
    1. matched join: one `ExecRow` + one `CommissionRow` (same `exec_id`) → one object with numeric
       `commission`, string `commission_currency`, numeric `realized_pnl`.
    2. unmatched exec (no commission row) → `commission` / `commission_currency` / `realized_pnl` = `null`.
    3. `realized_pnl` sentinel: commission with `realized_pnl: Some(1.7976931348623157e308)` → `null`;
       `realized_pnl: None` → `null` (via `pnl_number`).
    4. order preserved across ≥2 execs; `side` string passes through verbatim (`"BOT"`/`"SLD"`).
    5. orphan commission (`exec_id` matches no exec) → dropped, no phantom row; `merge_executions([], [])`
       → `[]`.
- NOT frozen — reviewed-by-reading + operator live acceptance (gateway reopens on `:4001`):
  `connect`/`resolve_account` reuse, `client.executions()` wiring, the ibapi-item→row extraction,
  drain-to-End, JSON assembly, `--format table`. Acceptance: on a day with fills, `omi --live executions`
  → itemized fills with `price` (+ `commission`/`realized_pnl` if the gateway sends them); `[]` on a flat day.

## Decisions

- A1 **Mirror the `pnl.rs`/`account.rs` split.** Gateway fn (`executions`) + pure seam
  (`merge_executions`). Reuse `connect`, `resolve_account`, `emit_success`, generic `render_table`,
  and `pnl_number`. No new abstraction, no new dependency.
- A2 **Drain to End via `iter_data()`, NOT take-first.** `Subscription<Executions>` terminates:
  `StreamDecoder<Executions>` maps `ExecutionDataEnd → Error::EndOfStream`
  (`src/orders/common/stream_decoders.rs:78`), so `iter_data()` yields items then `None` — the
  `orders`/`positions` shape, the **opposite** of `reqPnL` (ADR 0007). → **ADR 0008.**
- A3 **Best-effort commission JOIN by `exec_id`; missing → `null`.** `CommissionsReport` carries no
  request_id/order_id; ibapi routes it `ByExecutionId` (`routing.rs:132`) via an exec_id→subscription
  mapping stored when the matching `ExecutionData` was routed (`ExecutionData` strategy: "Store
  execution_id mapping", `routing.rs:129`). So commissions land in the same subscription and correlate to
  their execution. Only commissions arriving before `ExecutionDataEnd` are joined; the rest → `null`
  (not an error). → **ADR 0008.**
- A4 **`side` = wire string `"BOT"`/`"SLD"`** via `ExecutionSide::as_str`, not Rust `Debug` — stable
  agent key (same rule as machine-parseable output elsewhere).
- A5 **`time` emitted raw** (IB server-time string), no reformatting; deterministic, agent-first.
- A6 **Empty result is success** — `{ account, executions: [] }`, exit 0. A flat day is a valid answer.
- A7 **Read-only, `--live` allowed, no write gate** (ADR 0005). No filter flags this card
  (`ExecutionFilter` default except `account_code`); `--symbol`/`--side` deferred to `executions-filters`.

## Reused / new ADRs

Reused unchanged: 0001 (TWS via ibapi), 0002 (sync client), 0003 (stateless connect-per-command),
0004 (JSON-first + structured errors), 0005 (paper-default / live-opt-in; read-only on live allowed, no
write gate), 0006 (lib/bin split → freeze coverage). Reused seam: `pnl_number` (pnl-command). New:
**0008** (drain the executions stream to End; best-effort commission join by exec_id).
