# review-05 — phase1-readonly / PR #1 MERGED

Verdict: approved + merged
Time: 2026-06-28T13:38:17Z
PR: https://github.com/jackypanster/oh-my-ib/pull/1 (squash-merged → main `1c36b7c`)
Reviewer/merger: claude-opus-4-8 (pipeline-review merge step), operator-authorized

## Gates (all green)
- review-04 approved the diff (freeze gate empty, full verify + clippy green).
- Operator-confirmed live read-only acceptance (below) + explicit merge authorization.
- Squash-merged PR #1; feature branch deleted.

## Live read-only acceptance (operator choice: LIVE, not paper)
Run against a **Tiger Brokers Gateway** (TWS-API-compatible, JTS-based) on `127.0.0.1:4001`,
live account (id redacted — public repo), structurally read-only (no order paths; Read-Only API).
Required env: `IBAPI_TIMEZONE_ALIASES=HKT=Asia/Hong_Kong` (see finding A).

PRD criteria 1–8 — all PASS:
1. health → connected, server v221, account list returned.
2. account → stable keys account/net_liquidation/total_cash/buying_power/available_funds/currency (USD).
3. positions → `[]` (flat account; passed on retry, see finding B).
4. orders → `[]`.
5. quote AAPL --md-type delayed → `delayed:true` + delayed price ticks.
6. contract AAPL → conid 265598, APPLE INC, SMART, USD, Stock.
7. history AAPL --bar 1d --duration 1M → structured OHLCV + wap + count bars.
8. account --format table → renders.
Failure path: dead port → `{"error":{"code":"connection"}}` exit 2.

## Follow-ups (deferred to a new pipeline feature — NOT blockers; Phase 1 met its spec)
- A (important for HK/Tiger users): omi cannot connect without the `IBAPI_TIMEZONE_ALIASES=HKT=...`
  env — rust-ibapi rejects the gateway's "HKT" timezone at handshake. Consider auto-registering
  common unambiguous aliases (HKT→Asia/Hong_Kong) at startup, or documenting the env var.
- B (reliability): back-to-back `account_updates` commands with the same client_id can hit a transient
  `EAGAIN` (gateway hadn't released the prior subscription). Consider an EAGAIN retry or client_id rotation.
- C (data quality): `quote` `DelayedVolume` appears unscaled and `DelayedOpen`=0 in the snapshot —
  verify tick mapping/scaling.

Decision: Phase 1 (read-only daily driver) SHIPPED. A/B/C → new feature (Phase 1.1).
