# CONTEXT — pnl-command

New domain terms for this feature. Base glossary: `.pipeline/phase1-readonly/CONTEXT.md` (ground all
shared terms there; do not invent synonyms). Only the deltas live here.

## PnL domain

- **PnL** — profit and loss. This feature reports it at the **account level** via `reqPnL`.
- **Daily PnL** — the account's profit/loss **for the current trading day** (change since the prior
  close / session start). The headline monitoring number; *cannot* be derived from `account` or
  `positions` (it needs the day's starting NAV / realized fills), so it requires `reqPnL`. `ibapi` field
  `PnL.daily_pnl: f64`.
- **Unrealized PnL** — open-position profit/loss not yet locked in (mark-to-market since inception).
  Also visible per-position on `omi positions`; here it is the **account total**. `PnL.unrealized_pnl:
  Option<f64>`.
- **Realized PnL** — profit/loss locked in by closing trades. `PnL.realized_pnl: Option<f64>`.
- **reqPnL** — the TWS API request behind `client.pnl(account, model_code)`. Returns an **unbounded
  stream** of `PnL` snapshots with **no `End` marker** (see ADR 0007). `omi pnl` takes the first reading.
- **Unset sentinel** — IBKR encodes "no value / not applicable" as `Double.MAX_VALUE`, identical to Rust
  `f64::MAX` (`1.7976931348623157e308`). It arrives as a real `f64`, not null. `omi pnl` maps it (and any
  non-finite value) to JSON `null` via the pure `pnl_number` seam (operator decision A). Same hazard
  class as the unreliable delayed-volume tick that `quote-drop-volume` dropped.

## Conventions (feature-specific)

- JSON contract: `{ account, daily_pnl, unrealized_pnl, realized_pnl }` (snake_case, mirrors
  `account.rs`). No `currency` field (reqPnL returns none; base currency is on `omi account`).
- Read-only, no `OMI_ALLOW_LIVE` write gate; `--live` permitted (read-only on live is allowed, ADR 0005).
