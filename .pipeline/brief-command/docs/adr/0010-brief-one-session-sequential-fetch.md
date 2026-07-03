# ADR 0010 — brief: one session, strictly sequential multi-dataset fetch

Status: accepted (arch, 2026-07-03). Feature: brief-command. Builds on ADR 0003 (connect-per-command),
ADR 0007 (take-first), ADR 0008 (drain-to-End families), ADR 0009 (two-phase sweep on one session).

## Context

`omi brief` fetches six datasets (summary, account PnL, per-position PnL, positions, open orders,
executions) plus `server_time` on ONE `ibapi` sync connection. The PRD's load-bearing risk: is the
full interleaving safe on one session? Verified against ibapi-3.1.0 routing
(`src/transport/routing.rs`), which dispatches every incoming message to exactly one domain:

| dataset | request | routing domain (verified) |
|---|---|---|
| resolve_account | `managed_accounts()` | SharedMessage(`ManagedAccounts`) (`routing.rs:74-79`) |
| as_of | `server_time()` | SharedMessage(`CurrentTime`) one-shot (`accounts/sync.rs:29-37`) |
| summary+positions+discovery | `account_updates(&account)` | shared_request(`RequestAccountData`) — message-type domain (`accounts/sync.rs:224-228`); End marker exists (`AccountUpdate::End`, `accounts/mod.rs:251-260`) |
| account PnL | `pnl(&account, None)` | request-id (`request_with_id`, `accounts/sync.rs:131-133`); markerless → take-first (ADR 0007) |
| per-position PnL | `pnl_single(...)` ×N | request-id (ADR 0009); markerless → take-first |
| open orders | `all_open_orders()` | shared channel (`send_shared_request(RequestAllOpenOrders)`, `orders/sync.rs:27-32`); `OpenOrder`/`OrderStatus` route OrderOrShared, `OpenOrderEnd` → `EndOfStream` (`routing.rs:127-136`, `stream_decoders.rs:70-72`) |
| executions | `executions(filter)` | request-id (`orders/sync.rs:144-151`); `ExecutionData` stores an exec-id mapping, `CommissionsReport` routes **ByExecutionId** back to the same subscription (`routing.rs:127-136`); `ExecutionDataEnd` → `EndOfStream` (`stream_decoders.rs:79-92`) |

The one cross-domain overlap: `CommissionsReport` appears in BOTH the `Orders` and `Executions`
decoder message sets (`stream_decoders.rs:57-64,79-83`). Routing resolves it ByExecutionId (never to
the open-orders shared channel), so it cannot be stolen — but only exec-id mappings from a LIVE
executions subscription exist; interleaving two order-domain subscriptions is still pointless risk.

## Decision

1. **One connection; every subscription fully consumed and dropped before the next request starts**
   (drain-to-End for marker streams, take-first for markerless; `Drop` → `cancel()`,
   `subscriptions/sync.rs:284-289`). No concurrent subscriptions, ever.
2. **Fetch order (fixed):** `managed_accounts` (resolve) → `server_time` (as_of) →
   `account_updates` drain (feeds 3 sections, ADR 0011) → `pnl` take-first → N×`pnl_single`
   take-first (discovery order) → `all_open_orders` drain → `executions` drain.
3. **Fail-fast, no partial** (PRD D3): the first failed fetch aborts the command with the
   structured error envelope; nothing on stdout.
4. **Fallback deform (last resort, NOT implemented now):** if live acceptance shows a specific
   adjacency wedging the Tiger gateway, group the offending request into its own internal
   sequential session (distinct `client_id`), same process, same JSON contract. Record the
   observed pair in the journal before reaching for this.

## Consequences

- The intra-session EAGAIN class disappears for the daily flow: the documented race
  (`src/ib/mod.rs:38-48`) is a *reconnect* artifact (same client_id, back-to-back processes);
  within one connected session there is no reconnect. `connect-retry` stays for the siblings.
- Latency = one connect + Σ(sequential reads); N-position sweep cost already accepted
  (pnl-by-position D2). Blocking-read posture unchanged: `next_timeout` remains the recorded
  fallback (ADR 0007/0009), not the default.
- Unsolicited order-domain messages after the orders subscription is dropped (e.g. an
  `OrderStatus` from a fill) route OrderOrShared → no live channel → dropped by the bus;
  harmless, verified routing behavior.
