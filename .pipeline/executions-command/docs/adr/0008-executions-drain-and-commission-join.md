# ADR 0008 — Drain the executions stream to End; best-effort commission join by exec_id

Status: accepted

## Context
`omi executions` requests current-day fills via `ibapi` sync
`client.executions(filter) -> Subscription<Executions>` (`src/orders/sync.rs:144`). Two stream properties
had to be pinned against the crate source before locking the impl — the analog of ADR 0007's PnL trap.

1. **Termination.** Unlike `reqPnL` (unbounded, no marker — ADR 0007), this stream **is bounded**:
   `StreamDecoder<Executions>` (`src/orders/common/stream_decoders.rs:78`) maps
   `IncomingMessages::ExecutionDataEnd → Err(Error::EndOfStream)`, which `Subscription::iter_data()`
   surfaces as iterator termination (`None`). This is the `orders`/`positions` drain-to-End shape.

2. **Commission delivery.** A `CommissionReport` (`CommissionsReport`, msg 59) carries **no**
   request_id and **no** order_id — only an `exec_id` at the top level (`messages.rs:958`). ibapi routes
   it with `OrderRoutingStrategy::ByExecutionId` (`transport/routing.rs:132`): when the matching
   `ExecutionData` was routed (strategy `ExecutionData`, `routing.rs:129` — "Try order_id channel, then
   request_id channel. **Store execution_id mapping**"), the bus recorded an `exec_id → subscription`
   entry, and the commission report is delivered to that same subscription. So `ExecutionData` and its
   `CommissionReport` arrive **interleaved on one subscription**, correlated by `exec_id` — this is why
   `Executions` has both variants and why the JOIN key is `exec_id`.

The open question the crate cannot answer offline: does every `CommissionReport` arrive **before**
`ExecutionDataEnd`? In real IBKR TWS commission reports are interleaved with the exec details; the
operator's gateway is **Tiger** (TWS-API-compatible, not IBKR) and may (a) omit commission reports
entirely or (b) order them differently. `iter_data()` stops at the first `EndOfStream`, so any commission
delivered *after* `ExecutionDataEnd` is not observed.

## Decision
**Drain the subscription to End via `iter_data()`** (not take-first), accumulating both item kinds as they
arrive: `Executions::ExecutionData → ExecRow`, `Executions::CommissionReport → CommissionRow`. After the
drain, **join in the pure `merge_executions` seam by `exec_id`**:

- execution with a matching commission → numeric `commission` / `commission_currency` /
  `realized_pnl` (the last through `pnl_number`).
- execution with **no** matching commission (never sent, or arrived after End) → those three fields =
  **`null`**. Not an error.
- orphan commission (`exec_id` matches no execution) → **dropped**, no phantom row.
- no fills at all → `executions: []`, exit 0.

## Rationale
- Drain-to-End is the correct, safe shape here: a terminator exists, so there is no take-first hang risk
  (contrast ADR 0007), and executions always precede their `ExecutionDataEnd`, so **no fill is ever lost**.
- The `null` fallback makes the command **correct regardless of commission timing or availability** — the
  load-bearing property given a non-IBKR gateway. Tiger omitting commissions → useful rows with `null`
  commission fields; Tiger ordering commissions oddly → same. The command never crashes and never invents
  a commission.
- Keeping the join in the ibapi-free `merge_executions` seam makes the one piece of real logic frozen and
  offline-testable, mirroring `pnl_number` / `quote_price_tick`.
- v1 does **not** add a post-End read window / timeout to chase late commissions: unjustified complexity
  until live acceptance proves Tiger emits commissions after `ExecutionDataEnd` **and** the operator needs
  them.

## Consequences
- **Live-acceptance signal:** if `omi --live executions` on `:4001` shows `commission`/`realized_pnl`
  consistently `null` on a day with known commissions, the remedy is a bounded post-End read
  (`next_timeout(Duration)`) to collect trailing commission reports — recorded here as the fallback, not
  the default (same posture as ADR 0007's `next_timeout` note).
- Any future fill-level or `--watch` command reuses this drain-to-End + best-effort-join shape; the
  markerless take-first pattern (ADR 0007) is **not** applicable here.
- `exec_id` corrections (`.01`→`.02`) are distinct rows; a commission correction joins to whichever
  `exec_id` it names. Correction-collapsing remains out of scope.
