# arch — live-combo-unlock

Stage: arch · feature: live-combo-unlock · repo: jackypanster/oh-my-ib · branch: main
Extends ADR 0030. Decision record: `docs/adr/0031-live-combo-unlock.md`. Domain: `CONTEXT.md`.

## Chosen shape

One pure seam + a re-export + a call-site rewire in `option_combo`. The seam decides "is this a clean
2-leg 1:1 vertical, and if so what is its pure-width max risk?"; the rewire feeds that risk into the
already-frozen cap check and keeps the already-frozen gate. No new env, no new struct, no new command,
no state, no gateway round-trip added. Every refuse is offline-deterministic ⇒ freezable.

```
option_combo (trade.rs:799)                      [gateway fn — review-by-reading]
  parse legs → specs:Vec<LegSpec>  (:821-828)    unchanged
  same-underlying + qty + limit validation       unchanged (:829-848)
  if !cfg.preview {                               (:852)  rewire block ↓
    let is_live = cfg.port == LIVE_PORT;
    if is_live {
      combo_live_max_risk(&specs, args.qty)?   ── NEW pure seam ── Ok(risk) | Err(not-a-vertical)
        → resolve_max_notional(env OMI_MAX_NOTIONAL)?      REUSED (ADR 0030, frozen elsewhere)
        → check_live_write_posture(true, false, Some(risk), cap)?   REUSED (ADR 0030, frozen elsewhere)
    }                                             every Err → AppError::config (exit 5)
    require_live_write_gate(cfg)?;                unchanged  (:857) — posture BEFORE gate (deliberate)
  }
  connect → per-leg conid resolve → build_combo_order → place   unchanged (:860+)
```

Deleted: the single `refuse_live_combo_on_live(...)` call at `trade.rs:856`. The FN stays defined at
`:247` + re-exported at `mod.rs:45` (frozen by `tests/live_write_guardrail.rs` — do NOT delete/edit).

## Component boundaries / write-set (for the impl card)

- `src/ib/trade.rs` — ADD `pub fn combo_live_max_risk(specs: &[LegSpec], qty: f64) -> Result<f64,String>`
  (pure; the 7-condition predicate of ADR 0031 §D3 + `width×100×qty`; `Err(reason)` per condition).
  REWIRE `option_combo`'s `if !cfg.preview` block per the shape above (replace the one lockout line).
- `src/ib/mod.rs:45` — add `combo_live_max_risk` to the `pub use trade::{…}` list.
- `src/cli.rs` — help text ONLY if the combo command help currently states "combo is paper-only"; update
  to "live: 2-leg vertical spreads only, risk ≤ OMI_MAX_NOTIONAL". (Impl greps; skip if absent.)
- `tests/live_combo_unlock.rs` — NEW frozen spec (written by pipeline-task, not impl).

Out of write-set (MUST stay unchanged): `refuse_live_combo_on_live` body, `check_live_write_posture`,
`resolve_max_notional`, `require_live_write_gate`, `place_core`, `place_with_client`, `option_close`,
`cancel`, `shape_preview`, all paper/preview paths, and `tests/live_write_guardrail.rs` (byte-identical).

## Freeze plan (for pipeline-task)

FREEZE ONLY `combo_live_max_risk` in NEW `tests/live_combo_unlock.rs` (import via
`oh_my_ib::ib::combo_live_max_risk`; construct `LegSpec` literals — fields are `pub`). Assert:

- Ok: put credit vertical `[SELL 1 SYM E 185 P, BUY 1 SYM E 180 P]` qty 1 ⇒ `Ok(500.0)`; call vertical
  `[BUY 1 SYM E 250 C, SELL 1 SYM E 240 C]` qty 2 ⇒ `Ok(2000.0)`; debit vertical `[BUY 1 SYM E 180 P,
  SELL 1 SYM E 185 P]` qty 1 ⇒ `Ok(500.0)` (same width as the credit ⇒ premium-proof/credit-debit unified).
- Err (each a distinct not-a-vertical reason): 1 leg (`len<2`); 3 legs; diff expiry; diff right
  (one C one P); same action (both BUY); ratio 2 on a leg; equal strikes; diff underlying symbol.

NOT frozen: the `option_combo` wiring (review-by-reading, ADR 0031 §Freeze-coverage) and the within-cap
→ actually-place path (operator live acceptance — would place a real order). No re-freeze of any existing
spec. Single spec-rev for this feature.

## Reference-behavior artifact

NOT APPLICABLE. This feature adds NO new external-reference dependency: the risk decision is pure
arithmetic over `LegSpec` fields already parsed by the (shipped, paper-accepted) combo path; the gateway
BAG placement is unchanged from `option-combo` (ADR seq=8, paper-accepted). Nothing here depends on
un-probed third-party/wire semantics ⇒ no risk register, no tabled `⚠️` rows.

## Verify (unchanged 4 gates)

`cargo build` · `cargo test` (full; `tests/live_write_guardrail.rs` still 100% green) ·
`cargo clippy --all-targets -- -D warnings` (`refuse_live_combo_on_live` stays warning-free: pub +
re-exported + referenced by the frozen test). Offline refuse checks need NO gateway. Live-pass = operator
first live combo (NVDA 185/180) post-merge.
