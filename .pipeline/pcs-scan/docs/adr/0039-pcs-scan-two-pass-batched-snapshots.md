# ADR 0039 — pcs-scan: two-pass batched snapshots, skip-not-fail

Status: accepted
Builds on: ADR 0038 (total-deadline snapshot drain), ADR 0019 D3 (model greeks rows), ADR 0028 (chain rows).

## Context

PRD scope sketched "fetch every put in [0.70×spot, spot]". Measured 2026-09-26 (`omi --live option-chain`, union
strikes, 80–100% band alone): SPY 154, QQQ 152, GLD 79, SNDK 75 strikes ⇒ the 38-symbol list needs 1000+
option snapshots ⇒ 20+ batches × ≈11s IB tail ⇒ well over the PRD goal (≈2–3 min; acceptance ≤ ~5 min).
Most of those strikes are nowhere near |Δ| 0.20.

ibapi-3.1.0: `Subscription<T>` owns `Arc<dyn MessageBus>` + its own `request_id` and does not borrow the
`Client` (`subscriptions/sync.rs:38-49`; `market_data/realtime/sync.rs:186-196`) ⇒ many snapshot
subscriptions can be open at once on one blocking client; each buffers on its own request-id channel.

## Decision

1. **Batched snapshots.** One helper `snapshot_batch(client, contracts) -> Vec<Result<Ticks, SkipReason>>`:
   subscribe up to `PCS_BATCH_LINES = 50` contracts, then drain each with the ADR 0038 loop against ONE shared
   batch deadline (`batch_start + SNAPSHOT_DEADLINE`) — they run in parallel server-side, so the batch costs
   ≈ one IB snapshot tail. Per-contract `Some(Err)` or timeout ⇒ that contract's `Err(quote_error)`, never a
   command failure. All subscriptions are dropped (CancelMktData if unfinished) before the next batch.
   50 < IB's default 100 market-data lines, leaving headroom for other sessions.
2. **Two passes per scan (all symbols batched together per pass):**
   - Spot pass: stock snapshots for all symbols → `pick_spot`.
   - Chain step (sequential, same connection): conid resolve + `option_chain` → SMART row with
     `trading_class == symbol` (else the SMART row with most expirations) → `pick_expiry`.
   - Pass 1 (coarse): `coarse_strikes(strikes, spot, 8)` = up to 8 listed strikes spread evenly by index over
     [0.65×spot, spot] (both ends included when listed).
   - Pass 2 (refine): `estimate_target_strike(pass1_rows, target_delta)` — linear interpolation of |Δ| over
     strike between the adjacent pass-1 rows that bracket the target (no bracket ⇒ the pass-1 strike whose |Δ|
     is nearest). `refine_strikes(strikes, k_star, width, 3)` = the 3 listed strikes nearest k* PLUS each
     one's `strike − width` partner when listed, minus strikes already fetched.
   - Select over pass1 ∪ pass2 rows with `select_pcs` (PRD criterion 3/4 rules).
   Budget: ≤ 8 + 6 = 14 contracts/symbol ⇒ ≤ 532 for 38 symbols ⇒ ≤ 11 option batches + 1 spot batch ≈
   12 × ~11–20s ≈ 2.5–4 min (+ chain step ≈ 1 min).
3. **Skip-not-fail.** Usage errors (pre-connect) and connect failure fail the command; everything per-symbol or
   per-contract becomes a `skipped` row with a closed-set reason (PRD criterion 4). Precedence per symbol:
   `spot_unavailable` → `no_expiry_in_window` → `quote_error` (chain or every pass-1 contract failed) →
   `no_greeks` → `no_long_leg` → `below_min_credit`.

## Consequences

- Extra pure seams (`coarse_strikes`, `estimate_target_strike`, `refine_strikes`) — all hermetic, frozen.
- A symbol whose 0.20-delta strike is below 0.65×spot (extreme IV) gets its lowest coarse strike as k* ⇒
  candidate is the nearest available |Δ| (reported delta shows the gap). Accepted.
- Wall time dominated by batch count × IB tail; worst case bounded by batches × 20s.
