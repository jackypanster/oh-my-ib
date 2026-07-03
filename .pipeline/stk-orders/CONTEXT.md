# CONTEXT — stk-orders

New domain terms. Base glossary: `.pipeline/phase1-readonly/CONTEXT.md`; reuses **bounded
wait / Instant-classified None** (ADR 0016), **take-first timeout const** (ADR 0012). Deltas:

## Write-path domain

- **Write path** — the three verbs `buy`/`sell`/`cancel`; ALL write calls contained in
  `src/ib/trade.rs` (containment rule, ADR 0017). Everything else stays read-only.
- **Double gate** — live writes need `--live` (or any route to port 4001) AND
  `OMI_ALLOW_LIVE=1`; checked BEFORE connect (offline-frozen). Paper `:4002` = ungated
  sandbox.
- **Effective live port rule** — the gate triggers on `cfg.port == LIVE_PORT`, not on the
  flag, so `--port 4001` cannot bypass.
- **First ack** — the first `OrderStatus` or `OpenOrder` event on the order-id-routed
  `place_order` subscription; its status string is the ack. ExecutionData/CommissionReport
  are skipped (window refreshes).
- **UNKNOWN state** — a placement/cancel wait that timed out: the order MAY exist. The
  envelope names the allocated order id + `omi orders`; blind retry is forbidden (double-order
  bug). No auto-retry exists anywhere.
- **Ack object** — exact 6 keys: `order_id, status, symbol, action, quantity, limit_price`
  (null for MKT).

## Conventions (feature-specific)

- LMT ⇔ `--limit <px>` present; MKT ⇔ absent. TIF always DAY in v1.
- Local validation before anything: quantity > 0, limit > 0 when present (usage envelope).
- Acceptance is PAPER-only (criterion 11); the pipeline never places a live order.
- Red-line docs (AGENTS.md/CLAUDE.md) amended verbatim per arch.md §Docs amendment — the
  ONLY lines those files change.
- Review polarity flip: grep `place_order|submit_order|encode_place_order` — hits allowed
  ONLY in `src/ib/trade.rs`.
