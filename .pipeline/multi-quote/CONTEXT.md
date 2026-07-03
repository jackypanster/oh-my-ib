# CONTEXT — multi-quote

New domain terms for this feature. Base glossary: `.pipeline/phase1-readonly/CONTEXT.md`; reuses
**sequential fetch discipline** (brief-command), **routing domain** (brief-command), **bounded
drain / SnapshotEnd** (phase1 quote). Only the deltas live here.

## Batch-quote domain

- **Batch snapshot** — N symbols fetched sequentially on ONE connection with ONE
  `switch_market_data_type`, each a bounded `SnapshotEnd` drain, consume-then-drop before the
  next request (ADR 0013; ADR 0010 discipline).
- **`quote_one`** — the per-symbol seam: `(client, symbol, exchange, currency, delayed)` →
  exactly the pre-variadic single-symbol object `{symbol, delayed, ticks{…}}`. Gateway-dependent,
  review-by-reading.
- **`shape_quotes`** — the pure FROZEN N-shaping seam: 1 row ⇒ bare object (byte-identity red
  line), 2+ ⇒ bare array in input order, 0 ⇒ `[]` (defensive; clap makes it unreachable).
- **Input-order rule** — output rows follow the argument order verbatim; duplicates pass
  through (the agent owns its list).
- **Symbol-bearing context** — batch error contexts are `quote/<symbol>` so fail-fast names the
  offender (accepted N=1 failure-path delta, ADR 0013 Consequences).

## Conventions (feature-specific)

- `omi quote AAPL` (N=1): output byte-identical to pre-feature; `omi quote AAPL MSFT` (N≥2):
  bare JSON array. No wrapper object, no new keys.
- Whole-command fail-fast (D3): first failing symbol aborts; stdout stays empty; agent degrades
  to per-symbol invocations.
- Shared flags (`--sec-type`/`--exchange`/`--currency`/global `--md-type`) apply to the whole
  batch; STK-only guard rejects before connecting.
- Snapshot drains are bounded — NOT the ADR 0012 take-first class; no timeout wrapping.
- **Merge gate (PRD criterion 9)**: operator live-accepts single + batch in the SAME gateway
  session and cross-checks row shape before the PR merges.
