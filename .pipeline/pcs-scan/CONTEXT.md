# CONTEXT — pcs-scan

Deltas only. Reuses **snapshot drain / SNAPSHOT_DEADLINE / total-deadline loop** (ADR 0038), **model greeks
row** (ADR 0019 D3, `GreeksRow`), **chain row** (ADR 0028).

- **PCS (put credit spread)** — SELL 1 put at the short strike + BUY 1 put at `short − width`, same expiry;
  opens for a net credit; max loss = (width − credit) × 100.
- **Short / long leg** — the sold (higher strike) / bought (lower strike) put.
- **Spot** — underlying price chosen by `pick_spot` from a stock snapshot (Last → Bid/Ask mid → Close; the
  `Delayed*` labels are treated identically).
- **Target delta** — `--delta` (0.20); the short leg is the fetched put with |model Δ| nearest it.
- **Coarse / refine pass** — pass 1 fetches ≤8 evenly spread strikes to learn the Δ curve; pass 2 fetches the 3
  strikes nearest the interpolated target strike k* plus their `−width` partners (ADR 0039).
- **k\*** — estimated strike where |Δ| = target, by linear interpolation between bracketing pass-1 rows.
- **Batch** — ≤ `PCS_BATCH_LINES` (50) concurrent snapshot subscriptions drained against one shared deadline.
- **Candidate / skipped** — every input symbol yields exactly one: a candidate row or `{symbol, reason}`.
- **Skip reasons** — `no_expiry_in_window`, `spot_unavailable`, `no_greeks`, `no_long_leg`, `below_min_credit`,
  `quote_error`.
- **Usable put** — a fetched put with bid > 0 and ask > 0 (mid = (bid+ask)/2); only usable puts can be legs;
  only usable puts WITH a model delta can be the short leg.
- **return_on_risk** — `credit_mid × 100 / max_loss`; the sort key (desc).
