# CONTEXT — live-combo-unlock (domain language)

Ubiquitous terms this feature adds/sharpens. Ground for the cold task/impl/review nodes.

- **combo lockout** — ADR 0030 D4's categorical refusal of ALL live `option-combo`
  (`refuse_live_combo_on_live`, `trade.rs:247`). This feature REPLACES its call site with a risk breaker;
  the fn itself stays (frozen by another card — see *frozen-elsewhere*).
- **clean 2-leg 1:1 vertical** — the ONLY combo shape admitted onto the live path: exactly 2 legs, same
  underlying, same expiry, same right, opposite actions, both ratio 1, distinct strikes. Anything else
  (calendar = diff expiry; diagonal/straddle = diff right; ratio/backspread = ratio≠1; condor/butterfly
  = ≥3 legs; cross-underlying = diff symbol) is **not-a-clean-vertical** ⇒ refused, still paper-only.
- **defined-risk** — a position whose max loss is bounded and known before the fact. A 1:1 vertical is
  defined-risk; a ratio/naked leg is not. Only defined-risk combos are live-eligible.
- **width** — `|strike_a − strike_b|` of the two legs (per-share). The vertical's collateral/margin base.
- **max_risk (pure-width)** — `width × 100 × qty`. The live breaker's risk number. **Premium-proof**: the
  operator-typed net limit is NOT an input, so a mistyped credit cannot corrupt it. It is an UPPER bound
  on true max loss for any 1:1 vertical (credit or debit) ⇒ conservative, never under-counts.
- **premium-proof** — a safety property: the risk decision reads only strikes/qty, never the typed price,
  so fat-fingering the price cannot widen the admitted risk. (Contrast ADR 0030's single-leg
  `compute_notional`, which DOES read `|limit|` — correct there, wrong for combos.)
- **cap** — `OMI_MAX_NOTIONAL` (default `DEFAULT_MAX_NOTIONAL` = `$500`), resolved by the reused
  `resolve_max_notional` (fail-closed on a bad value). Shared by single-leg notional AND combo max_risk;
  no separate combo cap. `max_risk > cap` ⇒ refuse; `== cap` ⇒ pass (`>` not `>=`).
- **posture-before-gate** — combo's refuse ordering: the shape/cap check runs BEFORE
  `require_live_write_gate`, because a shape/cap refusal holds regardless of `OMI_ALLOW_LIVE` (reporting
  the env gate first would misdirect). Deliberately unlike `place_core` (gate-first).
- **frozen-elsewhere** — `refuse_live_combo_on_live`, `check_live_write_posture`, `resolve_max_notional`
  are frozen in `tests/live_write_guardrail.rs` (ADR 0030). This feature REUSES the latter two and
  UNWIRES the first; it MUST NOT edit that spec file (AGENTS.md hard invariant).
- **the new seam** — `combo_live_max_risk(specs: &[LegSpec], qty: f64) -> Result<f64, String>`: the
  clean-vertical predicate + width×100×qty. The ONLY new frozen surface, tested in
  `tests/live_combo_unlock.rs`.

Unchanged domain (do not touch): the port gate (`require_live_write_gate`), the single-leg/STK
`place_core` path, `option-close` (exit — never blocked), `cancel`, `--preview` (read-only), and all
paper (`:4002`) behavior.
