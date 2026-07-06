# CONTEXT — live-write-guardrail

Glossary + domain model for the live write posture guardrail (ADR 0030). Extends the Phase-2 write
vocabulary (ADR 0017 gate).

## Terms

- **Port gate** (`require_live_write_gate`, `trade.rs:175`) — the EXISTING coarse guard: may this process
  write to live at all? Keys on `cfg.port == LIVE_PORT (4001)` + `OMI_ALLOW_LIVE=1`. Unchanged by this
  feature.
- **Write posture guardrail** (NEW, this feature) — the FINE guard: is this specific live order sane?
  Refuses not-LMT / over-cap / combo. Runs alongside the port gate, offline, before connect.
- **Effective-live** — `cfg.port == LIVE_PORT`. Catches both `--live` and a hand-set `--port 4001` (the
  same key the port gate uses). Paper (`:4002`) is NOT effective-live.
- **Opening order** — establishes/increases exposure: `buy`/`sell`/`option-buy`/`option-sell`/`option-
  combo`. The guardrail binds these.
- **Closing order** — reduces exposure: `option-close` (side derived from the held position's sign).
  EXEMPT from the guardrail — never block an exit.
- **Notional** — `total_quantity × |limit_price| × multiplier` (STK ×1, OPT ×100). A fat-finger magnitude
  proxy, NOT a true risk figure (for a spread the net limit under-counts risk — irrelevant here: combo is
  refused on live). `None` for MKT (no limit) — but a live MKT is refused before notional matters.
- **Cap** — the notional ceiling for live opening orders. `OMI_MAX_NOTIONAL` env if set (finite, > 0),
  else `DEFAULT_MAX_NOTIONAL = 500.0`. Per-command + auditable, mirroring `OMI_ALLOW_LIVE`. A bad value
  fails closed (refuse).
- **Fail-closed** — an unparseable/≤0 `OMI_MAX_NOTIONAL` REFUSES; it never silently reverts to the
  default. A typo must not widen risk.
- **Interlock posture** (operator, 2026-07-06) — STK + single-leg option may go live; combo stays paper.
  The guardrail's combo-lockout (D4) is this posture in code.

## Invariants

- The guardrail fires ONLY when effective-live AND not preview AND the verb is an opening order routed
  through `place_core` (or `option_combo`). Paper, preview, close, cancel ⇒ exempt by construction.
- Refuses are offline-deterministic (before connect) — reproducible with `:4001` down; no order possible.
- All refuses ⇒ `AppError::config` (code `config`, exit 5), uniform with the port gate.
- `require_live_write_gate` is extended alongside, never weakened; write code stays in `trade.rs`.

## Why offline / freezable (contrast ADR 0029)

Every guardrail decision is made from local inputs (port, order_type, qty, limit, env) before any
gateway contact, so the refuse paths are pure and FROZEN as unit tests — unlike the prior live-gate
fix, whose correctness ("no order when a gateway is up") could not be a unit assertion. Only the
within-cap → actually-place path stays operator live acceptance (asserting it would place a real order).
