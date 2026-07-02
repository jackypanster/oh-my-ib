# ADR 0009 — pnl-by-position sweep: sequential take-first reads, fail-fast, no partial output

Status: accepted

## Context

`omi pnl-by-position` needs per-position PnL for every position in the account. ibapi-3.1.0 offers
`client.pnl_single(&account, ContractId, Option<&ModelCode>) -> Subscription<PnLSingle>`
(`accounts/sync.rs:159`) — **one contract per subscription**, and like `reqPnL` (ADR 0007) the stream
is **markerless**: `StreamDecoder<PnLSingle>` handles only `[PnLSingle, Error]`
(`accounts/common/stream_decoders/mod.rs:53-58`) — no End message, ticks stream until cancelled.
The conid list must be discovered first; the repo already drains the `account_updates` portfolio
stream for exactly this data (`src/ib/positions.rs`, chosen there over `client.positions()` because
the portfolio stream carries valuation + symbol).

Two routing facts (verified in crate source) make the composition safe:
- `account_updates` is a **shared-channel** subscription, routed by message type
  (`shared_request(OutgoingMessages::RequestAccountData, ...)`, `accounts/sync.rs:224-228`).
- `pnl_single` is **request-id-routed** (`PnLSingleRequest{request_id}`,
  `cancel_by_id!(CancelPnLSingle)`, `accounts/common/encoders.rs:54-67`).
- `Subscription: Drop → cancel()` (`subscriptions/sync.rs:284-289`) — dropping unsubscribes.

## Decision

1. **Sequential two-phase sweep on ONE connection**: (a) discovery — drain `account_updates` to
   `AccountUpdate::End`, collect `(conid, symbol)` from every `PortfolioValue` (qty==0 included,
   PRD D6), drop the subscription; (b) per conid, in discovery order:
   `pnl_single(&account, ContractId::from(conid), None)` → **`next_data()` take-first** (ADR 0007) →
   drop. No concurrency, no subscription overlap.
2. **Fail-fast, no partial output**: any `Some(Err(_))` or `None` from any read aborts the whole
   command with `AppError::data` naming the failing conid. The absence/failure line: a missing
   **value** inside a successful reading is `null` (sentinel seam); a failed **request** is a
   structured error + non-zero exit (ADR 0004).
3. **Sentinel routing**: `daily_pnl`, `unrealized_pnl`, `realized_pnl`, and (defensively) `value`
   pass through the shared `pnl_number` seam — `PnLSingle` fields are bare `f64`
   (`accounts/mod.rs:172`), so IBKR's `f64::MAX` unset marker arrives as a value. `position`,
   `conid`, `symbol` are raw. A legitimate market value is never `f64::MAX`/non-finite, so routing
   `value` loses nothing and shields unproven Tiger data quality.
4. **Blocking reads stay the default**: `next_data()` blocks like every sibling command; the recorded
   fallback — applied only if live acceptance shows hangs — is `next_timeout(Duration)` per read
   (ADR 0007's fallback discipline). Note the risk multiplies by N here: one unresponsive conid
   hangs the whole sweep; that is accepted until live evidence says otherwise.

## Rationale

- Sequential-on-one-connection is the simplest shape that satisfies the stateless
  connect-per-command model (ADR 0003); the disjoint routing domains above make it provably safe,
  and the known Tiger EAGAIN quirk is a *reconnect* issue (handled at `src/ib/mod.rs` connect
  retry), not an in-session one.
- Fail-fast over per-row degradation: a partial sweep is indistinguishable from a complete one to
  the consuming agent, which would silently misreport "what moved today". The repo's global
  fast-fail rule and ADR 0004 already draw this line; `executions`' per-row nulls (ADR 0008) are
  NOT a precedent here — there the *join data* is legitimately optional, the request succeeded.

## Consequences

- Latency grows linearly with position count (accepted, PRD D2 — no filters).
- If live acceptance shows Tiger rejects `reqPnLSingle` while `reqPnL` works, the feature is
  rejected at review (PRD D3 gate) — that is the designed failure point, not a merge-then-revert.
- Any future concurrent/streaming variant (e.g. `--watch`) must revisit decision 1 explicitly; this
  ADR binds only the one-shot sweep.
