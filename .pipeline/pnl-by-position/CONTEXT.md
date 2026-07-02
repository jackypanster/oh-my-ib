# CONTEXT — pnl-by-position

New domain terms for this feature. Base glossary: `.pipeline/phase1-readonly/CONTEXT.md`; reuses
`pnl-command`'s **unset sentinel** and **take-first** terms, and `executions-command`'s pure-seam
convention. Only the deltas live here.

## Per-position PnL domain

- **Per-position PnL (`reqPnLSingle`)** — the TWS request behind
  `client.pnl_single(&account, ContractId, Option<&ModelCode>)` (ibapi `accounts/sync.rs:159`):
  a real-time PnL subscription for **one contract** in one account. Returns
  `PnLSingle{position, daily_pnl, unrealized_pnl, realized_pnl, value}` — all bare `f64`
  (`accounts/mod.rs:172`), so the unset sentinel arrives as a *value*, never as `None`.
  **Markerless stream** (`StreamDecoder<PnLSingle>` message set = `[PnLSingle, Error]`,
  `stream_decoders/mod.rs:53-58`) → **take-first** is binding (ADR 0007 Consequences, ADR 0009);
  a drain loop hangs forever.
- **Discovery** — the enumeration pass that yields the account's `(conid, symbol)` list: drain the
  `account_updates` portfolio stream to `AccountUpdate::End` (the `positions.rs` pattern). Includes
  qty==0 rows (closed-today positions carry today's realized PnL — PRD D6).
- **Sweep** — this feature's read shape: one discovery pass, then N **sequential** take-first
  `pnl_single` reads on the same connection, in discovery order, **fail-fast** on any read failure
  (ADR 0009). No partial output: a sweep either completes or errors.
- **Row (`PnlSingleRow`)** — the plain ibapi-free struct the pure seam consumes
  (`conid, symbol, position, daily_pnl, unrealized_pnl, realized_pnl, value`). `symbol` comes from
  discovery (PnLSingle carries no contract identity); `position`/`value`/PnL fields come from the
  PnLSingle reading (fresher than the portfolio snapshot).
- **Unset sentinel** — (reused) IBKR `Double.MAX_VALUE` == `f64::MAX` == `1.7976931348623157e308`;
  mapped to JSON `null` via the shared `pnl_number` seam. Applies to `daily_pnl`, `unrealized_pnl`,
  `realized_pnl`, AND (defensively) `value`; never to `position`/`conid`/`symbol`.

## Conventions (feature-specific)

- JSON contract: `{ account, by_position: [ { conid, symbol, position, daily_pnl, unrealized_pnl,
  realized_pnl, value } ] }` (snake_case; PnL/value fields `number | null`). Flat account →
  `by_position: []`, exit 0.
- **Absence vs failure**: a missing *value* is `null` (sentinel seam); a failed *request* is a
  structured `{"error":{...}}` + non-zero exit (ADR 0004). Never a partial sweep (ADR 0009).
- Read-only; `--live` permitted (ADR 0005); no filter flags (PRD D2 — agent filters client-side).
- **Merge gate (PRD D3)**: operator live-accepts `omi --live pnl` BEFORE this feature's PR merges,
  then `omi --live pnl-by-position` in the same session.
