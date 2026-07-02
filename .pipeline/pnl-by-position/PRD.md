# PRD — pnl-by-position

Feature: `omi pnl-by-position` — per-position Daily / Unrealized / Realized PnL in one read-only call.
Status: decision-complete (grilled 2026-07-02, operator locked D1–D3 below; code facts verified against
ibapi-3.1.0 crate source, not guessed).

## Problem

The agent cannot answer the highest-frequency daily question — "which position drove today's P&L" —
from any existing command: `omi positions` carries only inception-to-date `unrealized_pnl` per position
(portfolio stream), `omi pnl` is account-level only (its PRD explicitly deferred per-position PnL to
this feature). The gap: per-position **daily** and **realized** PnL.

## Goal

New read-only subcommand `omi pnl-by-position`: discover the account's positions (conid + symbol), take
one `ibapi` sync `client.pnl_single(&account, ContractId, None)` snapshot per position, and emit a
single JSON object joining identity to PnL. Stateless connect-per-command like every sibling.

## Success criteria (acceptance)

1. `omi pnl-by-position` (paper `:4002` default) exits 0 and prints
   `{"account":"<id>","by_position":[...]}` — one row per position with exactly:
   `conid`, `symbol`, `position`, `daily_pnl`, `unrealized_pnl`, `realized_pnl`, `value` (snake_case).
2. IBKR "no value" sentinel (`f64::MAX` = 1.7976931348623157e308) and any non-finite value render as
   JSON `null` in every PnL field (route through the existing `pnl_number` seam) — an agent never sees
   a sentinel as a P&L number.
3. Flat account (no positions) → `{"account":"<id>","by_position":[]}`, exit 0 (not an error).
4. Gateway down → non-zero exit + `{"error":{code,message,context}}` on stderr (existing contract).
5. `--format table` renders the rows human-readably.
6. `cargo build` · `cargo clippy --all-targets -- -D warnings` · `cargo test` green (offline frozen
   spec; the gateway-dependent layer is reviewed-by-reading per repo convention).
7. **Merge gate (operator, live — D3):** BEFORE this feature's PR merges, the operator must FIRST pass
   the outstanding live acceptance of account-level `omi --live pnl` (numeric `daily_pnl`, no sentinel
   leak) — proving the Tiger gateway serves the reqPnL family — then live-accept
   `omi --live pnl-by-position` in the same session (rows for held positions; `[]` when flat).

## Scope

- One new flat subcommand + one new gateway module mirroring `src/ib/pnl.rs`; conid/symbol discovery
  via the existing `account_updates` portfolio stream (same source as `positions`); then, per conid,
  one `pnl_single` take-first read (ADR 0007 pattern) on the SAME connection; emit; disconnect.
- All positions, no filter flags (v1).
- No new dependency.

## Non-scope (explicitly NOT this feature)

- No `--symbol`/`--conid` filter — the agent filters JSON client-side; same low-ROI call as the
  deferred `executions-filters`.
- No `--by-position` flag on `omi pnl` — rejected in grilling (D1): one command emitting two output
  shapes weakens the agent contract; the repo has no shape-switching-flag precedent (`--format` changes
  rendering, `--md-type` changes source, neither changes keys).
- No streaming / `--watch`; no `model_code` (pass `None`); no write path; no `OMI_ALLOW_LIVE`.

## Resolved decisions (locked)

- D1 **CLI = new flat subcommand `pnl-by-position`** (operator, grilled). Distinct command, distinct
  stable JSON shape — consistent with the existing 9 flat subcommands.
- D2 **All positions, no filter** (operator, grilled). The core use case is the full sweep; N
  sequential requests is acceptable cost for the stateless CLI model.
- D3 **Merge gate = live `omi --live pnl` acceptance first** (operator, grilled). Bounds the feature's
  most fragile assumption (unverified reqPnL-family support on the Tiger gateway) at the merge
  boundary instead of after merge. Gateway is currently closed; the gate waits for the operator.
- D4 **Take-first per `pnl_single` subscription** — `StreamDecoder<PnLSingle>` message set is
  `[PnLSingle, Error]` with NO End marker (verified ibapi-3.1.0
  `src/accounts/common/stream_decoders/mod.rs:53-58`); ADR 0007's Consequences already bind this
  feature to take-first. Drop each subscription after its first reading; a drain loop hangs forever.
- D5 **Sentinel mapping via existing `pub pnl_number`** — `PnLSingle` fields are bare `f64`, not
  `Option` (verified `src/accounts/mod.rs:172`): the sentinel arrives as a value, so EVERY PnL field
  routes through `pnl_number`.
- D6 **conid source = `account_updates` `PortfolioValue`** (the `positions.rs` pattern — carries
  conid + symbol + qty; `client.positions()` lacks valuation, see `src/ib/positions.rs` header).
  Rows with `position == 0` (closed-today) ARE queried — they carry today's realized PnL, in-scope
  for "what moved today". Whether Tiger emits qty-0 rows is a live-acceptance observation, not a
  blocker.

## Risks / fragile assumptions

- **LOAD-BEARING: Tiger gateway must serve `reqPnLSingle`.** Unverified — even account-level `reqPnL`
  awaits live acceptance (gateway closed). Bounded by D3. If `pnl` passes live but `pnl-by-position`
  does not, reject at review per the state machine (`attempts++` → impl/hunt); do not merge.
- N sequential `pnl_single` reads: latency grows with position count — accepted (D2). If a
  subscription never emits a first tick, `next_data()` blocks like every sibling; the recorded
  fallback is `next_timeout(Duration)` (ADR 0007) — fallback only, not default.
- Mid-session subscription sequencing: `account_updates` drain → drop → N × `pnl_single` on one
  client is a NEW interleaving for this codebase (known Tiger EAGAIN quirk lives at the connect
  layer, `src/ib/mod.rs`). Arch must confirm the crate's request-id isolation makes this safe.

## Verification

- Offline: frozen spec tests (task stage freezes; card-scoped runner per CONTRACT §Multi-card).
- Live (operator): the D3 sequence — `omi --live pnl` FIRST, then `omi --live pnl-by-position`.
