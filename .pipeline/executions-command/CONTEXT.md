# CONTEXT — executions-command

New domain terms for this feature. Base glossary: `.pipeline/phase1-readonly/CONTEXT.md` (ground all
shared terms there; do not invent synonyms). Reuses `pnl-command`'s **unset sentinel** term. Only the
deltas live here.

## Executions domain

- **Execution (fill)** — a single (possibly partial) fill of an order: shares transacted at a price at a
  time. `omi executions` reports the account's fills for the **current trading day only** (`reqExecutions`
  returns since-midnight, no history). Distinct from **order** (`omi orders` = still-working orders) and
  from **position** (`omi positions` = net holdings). This command fills the order-lifecycle gap between
  "working" and "net result".
- **exec_id (execution id)** — the unique id of a fill (`Execution.execution_id`). Each partial fill has
  its own. A **correction** reuses the id with a differing suffix after the final dot (`…​.01` → `…​.02`);
  v1 reports corrections as distinct rows (no collapsing — non-scope).
- **Commission report** — a separate TWS message (`CommissionReport`) carrying the commission, currency,
  and realized P&L **for one execution**, keyed by `exec_id`. It has **no** request_id/order_id; ibapi
  routes it `ByExecutionId` into the same subscription as its execution (see ADR 0008).
- **cumulative_quantity / average_price** — running fill quantity and average price for the order across
  its fills (`Execution.cumulative_quantity` / `.average_price`), commissions excluded.
- **side** — `"BOT"` (bought) or `"SLD"` (sold), the canonical IBKR wire string (`ExecutionSide::as_str`).
- **reqExecutions** — the TWS request behind `client.executions(filter)`. Returns a **bounded** stream:
  `ExecutionData` + `CommissionReport` items, terminated by `ExecutionDataEnd` → `Error::EndOfStream`
  (drain-to-End, unlike `reqPnL`; see ADR 0008). Scoped server-side by `ExecutionFilter.account_code`.
- **Unset sentinel** — (reused from `pnl-command`) IBKR's `Double.MAX_VALUE` == `f64::MAX`
  (`1.7976931348623157e308`) "no value" marker. A commission's `realized_pnl` is run through the
  `pnl_number` seam, mapping the sentinel / any non-finite / `None` → JSON `null`.

## Conventions (feature-specific)

- JSON contract: `{ account, executions: [ { exec_id, order_id, perm_id, time, symbol, conid, side,
  shares, price, cumulative_qty, avg_price, exchange, commission, commission_currency, realized_pnl } ] }`
  (snake_case; `commission`/`commission_currency`/`realized_pnl` are `null` when no commission report
  joined). Empty day → `executions: []`, exit 0.
- Read-only, no `OMI_ALLOW_LIVE` write gate; `--live` permitted (read-only on live is allowed, ADR 0005).
- No filter flags this card (`ExecutionFilter` default except `account_code`). `--symbol`/`--side`/`--days`
  are a future `executions-filters` card.
