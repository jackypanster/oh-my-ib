# PRD — brief-command

Feature: `omi brief` — the daily account snapshot (account summary + PnL + per-position PnL +
positions + open orders + today's executions) in ONE gateway connection, one stable JSON.
Status: decision-complete (grilled 2026-07-03, operator locked D1–D4 below; code facts verified
against this repo's source and the pnl-by-position ADRs, not guessed).

## Problem

The agent's single highest-frequency job — "how is the account today" — currently requires 6–7
sequential `omi` invocations (`account`, `pnl`, `pnl-by-position`, `positions`, `orders`,
`executions`, often `health` first). Each invocation opens its own gateway connection
(connect-per-command), so the daily flow:

- pays ~7 connect/handshake round-trips per question;
- triggers the documented Tiger-gateway race — back-to-back reconnects with the same `client_id`
  hit EAGAIN before the gateway releases the prior subscription (`src/ib/mod.rs:38-48`; the
  `connect-retry` feature only mitigates it with retry+backoff, it does not remove the class);
- yields a time-skewed picture (positions from t0, PnL from t3 seconds later) the agent must join
  from 6 separate JSON documents.

## Goal

New read-only subcommand `omi brief`: connect ONCE, fetch all six account-scoped datasets
sequentially on that single `ibapi` sync client, emit one composite JSON, disconnect. Kills the
EAGAIN class for the daily flow (no back-to-back reconnects), cuts ~7 connections to 1, and gives
a time-consistent snapshot in one agent-parseable document.

## Success criteria (acceptance)

1. `omi brief` (paper `:4002` default) exits 0 and prints ONE JSON object with exactly these
   top-level keys (snake_case):
   `account`, `as_of`, `account_summary`, `pnl`, `pnl_by_position`, `positions`, `orders`,
   `executions`.
2. **Section payloads are byte-shape-identical to their source commands** (same keys, same
   ordering rules, same `pnl_number` sentinel mapping — no new row shapes). Hoisting rule: the
   shared `"account"` key appears ONCE at top level; each section nests the source command's
   payload minus that wrapper:
   - `account_summary` = `account`'s object minus `account` (`net_liquidation`, `total_cash`,
     `buying_power`, `available_funds`, `currency`);
   - `pnl` = `pnl`'s object minus `account` (`daily_pnl`, `unrealized_pnl`, `realized_pnl`);
   - `pnl_by_position` = `pnl-by-position`'s `by_position` array;
   - `positions` = `positions`'s `positions` array;
   - `orders` = `orders`'s `open_orders` array;
   - `executions` = `executions`'s `executions` array.
3. `as_of` = the gateway's server time (ISO-8601 UTC string) — server truth, not the local clock.
4. Exactly ONE gateway connection per invocation (one `Client::connect`; verified by reading at
   review + observable at live acceptance).
5. **Fail-fast, no partial output**: any section fetch failing ⇒ non-zero exit +
   `{"error":{code,message,context}}` on stderr, NOTHING on stdout. A partial brief is
   indistinguishable from a complete one to the consuming agent (the `pnl_by_position` rule,
   repo-wide).
6. Quiet/flat account: empty arrays (`[]`) in array sections, exit 0 (not an error).
7. Gateway down → the existing connection-error contract (non-zero exit, structured stderr).
8. `--format table` renders each section as its own labeled block, human-readable.
9. `cargo build` · `cargo clippy --all-targets -- -D warnings` · `cargo test` green (offline
   frozen spec; the gateway-dependent layer is reviewed-by-reading per repo convention).
10. **Merge gate (operator, live):** before this feature's PR merges, the operator live-accepts
    `omi --live brief` and cross-checks its values for consistency against the individual commands
    run in the same session. (pnl-by-position D3 pattern.)

## Scope

- One new flat subcommand + one new gateway module (`src/ib/brief.rs`) orchestrating the six
  EXISTING fetch paths over one shared client. Requires the mechanical refactor of each source
  module into a `*_with_client(&Client, ...)`-style seam (public `fn x(cfg)` keeps its own connect
  and delegates) — reuse-by-refactor, zero duplicated fetch logic.
- Expected consolidation (arch to verify, see Risks): ONE `account_updates` drain feeds
  `account_summary` (AccountValue keys) + `positions` (PortfolioValue rows) + the
  `pnl_by_position` conid discovery — the stream carries both message kinds; today three separate
  commands each drain it independently.
- No new dependency.

## Non-scope (explicitly NOT this feature)

- No section include/exclude flags (`--no-executions` etc.) — fixed shape v1; the agent trims
  client-side (pnl-by-position D2 precedent: no filter flags, low-ROI).
- No `quote`/`history`/`contract` sections — symbol-parameterized, not account-snapshot data.
- No `positions ⋈ pnl_by_position` re-join into a merged array — rejected in grilling (D2): it
  invents a new row shape needing its own frozen spec; v1 nests both raw sections, redundancy is
  the agent's client-side concern.
- No `health` section — `as_of` carries the only meta the snapshot needs.
- No streaming / `--watch`; no retry-differently semantics; no write path; no `OMI_ALLOW_LIVE`.

## Resolved decisions (locked)

- D1 **Feature choice = `omi brief`** (operator, grilled 2026-07-03). Picked as highest-ROI over
  option-chains (options trading unproven for this account; entitlement risk; Phase-1 STK-only)
  and multi-symbol quote (optimizes a side flow, not the daily main flow). ROI evidence: kills the
  EAGAIN class at its cause; 7→1 connections on the highest-frequency query; all six data paths
  already live-proven on the Tiger gateway ⇒ near-zero API risk.
- D2 **Shape = verbatim nesting** (operator, grilled, preview-confirmed). Top-level
  `{account, as_of, + 6 sections}`; hoisting rule per success criterion 2. Zero new shaping logic.
- D3 **Whole-command fail-fast** (operator, grilled). Consistent with the repo's no-partial rule;
  on failure the agent degrades to the individual commands.
- D4 **Name = `brief`** (operator, grilled). `snapshot` collides with IB market-data snapshot
  terminology (`quote.rs`), `summary` with `account`'s "Account summary" help text.
- D5 **One connection, sequential fetch, shared-client seams** (code). Sections fetch in a fixed
  order on one client; each source module refactored to accept `&Client`. Fetch order and
  subscription lifecycle are arch's to pin (ADR), bound by ADR 0007 (take-first on markerless
  streams) and ADR 0009 (two-phase sweep) as prior art.
- D6 **`as_of` = gateway `server_time()`** (code — `health` already calls it; server truth over
  local clock).

## Risks / fragile assumptions

- **LOAD-BEARING: the full six-dataset interleaving on ONE session is new as a whole.** Pairwise
  prior art exists — ADR 0009 proved `account_updates` drain → drop → N×`pnl_single` live on
  Tiger — but `+ pnl + open_orders + executions` on the same session is unproven. Arch must
  verify per-pair safety in ibapi-3.1.0 source (request-id isolation, subscription cleanup on
  drop, any singleton subscriptions), and live acceptance (criterion 10) proves it on Tiger.
  **Recorded fallback deform** (last resort, arch decides): group conflicting requests into
  internal sequential sessions with distinct `client_id`s — still one process, one command,
  same JSON contract; costs elegance, not the contract.
- **`account_updates` is a singleton-style subscription** (subscribe/unsubscribe per account).
  Today three commands re-subscribe it back-to-back across separate connections — exactly the
  async-release window behind the EAGAIN quirk. The D5/Scope consolidation (single drain feeding
  three sections) removes the re-subscribe entirely; arch confirms the single-drain read of
  AccountValue + PortfolioValue in one pass against ibapi source.
- Latency: one connection, ~4 subscription reads + N `pnl_single` reads; N-position sweep cost
  already accepted (pnl-by-position D2). Blocking reads: same posture as every sibling
  (`next_timeout` fallback stays recorded in ADR 0007/0009, not default).
- Rollback: purely additive subcommand — revert = drop the command; no state, no data, no config
  migration.

## Verification

- Offline: frozen spec tests (task stage freezes; card-scoped runner per CONTRACT §Multi-card) —
  the pure seams: composite assembly/hoisting rule, fail-fast propagation, table rendering;
  offline-deterministic CLI contract against a dead port.
- Live (operator): criterion 10 — `omi --live brief`, cross-checked against individual commands
  in the same session.
