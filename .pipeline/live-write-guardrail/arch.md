# arch — live-write-guardrail

Stage: arch · decision of record: ADR 0030. Product change in `src/ib/trade.rs` only (+ `cli.rs` help).
The port gate (`require_live_write_gate`) is EXTENDED alongside, never weakened.

## Chosen shape

A write **posture guardrail**: four pure decision seams + thin gateway wiring. On the LIVE real-order
path, refuse (offline, before connect) any opening order that is not-LMT (D1), over the notional cap
(D2/D3), or a combo (D4). `option-close`/`cancel`/`--preview`/paper are exempt (D5). All refuses =
`AppError::config` (exit 5). See ADR 0030 for the full rationale + rejected alternatives.

## Write-set for impl

### FROZEN pure seams — `src/ib/trade.rs` (spec = `tests/live_write_guardrail.rs`)
- `compute_notional(quantity: f64, limit: Option<f64>, multiplier: f64) -> Option<f64>`
  — `limit.map(|l| quantity * l.abs() * multiplier)` (mirror `shape_preview`, `trade.rs:85`).
- `resolve_max_notional(raw: Option<&str>) -> Result<f64, String>`
  — `None ⇒ Ok(DEFAULT_MAX_NOTIONAL)`; `Some(s) ⇒` parse `f64`, require finite ∧ `> 0.0`, else
  `Err("invalid OMI_MAX_NOTIONAL '{s}': expected a positive number")`.
- `check_live_write_posture(is_live: bool, is_mkt: bool, notional: Option<f64>, cap: f64) -> Result<(), String>`
  — `!is_live ⇒ Ok`; `is_mkt ⇒ Err("live orders must be LMT — pass --limit (MKT is paper-only)")`;
  `match notional { Some(n) if n > cap ⇒ Err("live notional {n} exceeds cap {cap} — raise OMI_MAX_NOTIONAL to override"), _ ⇒ Ok }`.
  (On the live LMT path `notional` is always `Some`; the `is_mkt` arm has already handled `None`.)
- `refuse_live_combo_on_live(is_live: bool) -> Result<(), String>`
  — `is_live ⇒ Err("option-combo is paper-only during the trial (interlock posture) — use paper :4002")`; else `Ok`.
- `const DEFAULT_MAX_NOTIONAL: f64 = 500.0;`
- Re-export all four fns via `pub use trade::{...}` in `src/ib/mod.rs:45` (the frozen test imports
  `oh_my_ib::ib::{compute_notional, resolve_max_notional, check_live_write_posture, refuse_live_combo_on_live}`).

### WIRED (review-by-reading, NOT frozen) — `src/ib/trade.rs`
- `place_core` (`468`): after the `if cfg.preview { … return … }` branch and the existing
  `require_live_write_gate(cfg)?`, insert the posture check BEFORE `connect`:
  ```
  let is_live = cfg.port == LIVE_PORT;
  let cap = resolve_max_notional(std::env::var("OMI_MAX_NOTIONAL").ok().as_deref())
      .map_err(|m| AppError::config(m, ctx))?;
  let multiplier = if matches!(contract.security_type, SecurityType::Option) { 100.0 } else { 1.0 };
  let is_mkt = order.order_type == "MKT";
  let notional = compute_notional(order.total_quantity, order.limit_price, multiplier);
  check_live_write_posture(is_live, is_mkt, notional, cap).map_err(|m| AppError::config(m, ctx))?;
  ```
  (Ordering: gate → posture → connect. Both offline. Paper: `is_live=false` ⇒ posture is Ok. Preview:
  never reaches here — short-circuits above.)
- `option_combo` (`713`): on the real path only, before `require_live_write_gate` (site `766`):
  ```
  if !cfg.preview {
      refuse_live_combo_on_live(cfg.port == LIVE_PORT).map_err(|m| AppError::config(m, ctx))?;
      require_live_write_gate(cfg)?;
  }
  ```
- `std::env` / `SecurityType` / `LIVE_PORT` already in scope in `trade.rs` (env used by the gate;
  `SecurityType` imported line 20; `LIVE_PORT` line 25).

### `src/cli.rs`
- Update the `Buy`/`Sell` doc comments (and optionally the module note) to state: live orders must be
  LMT and are capped at `OMI_MAX_NOTIONAL` (default $500). No arg/flag changes.

### DO NOT TOUCH
`require_live_write_gate` body; `place_with_client` (would catch `option-close`); `option_close`;
`cancel`; `shape_preview` (extract the math into `compute_notional` but leave `shape_preview` calling its
own inline map, OR have it call `compute_notional` — impl choice, but its output JSON must be byte-identical
so the order-preview frozen tests stay green); any read command; `src/config.rs` (env read lives in
`trade.rs`, gate precedent — no `config.toml` key this card).

## Component boundaries / pipeline handling

- spec-paths: `tests/live_write_guardrail.rs` (NEW file — no other feature's frozen spec touched; no
  re-freeze). One freeze commit ⇒ one spec-rev.
- impl-paths: `src/ib/trade.rs`, `src/ib/mod.rs`, `src/cli.rs` (disjoint from spec-paths).
- verify (card-scoped): `cargo test --test live_write_guardrail`.
- full-verify (current.json): `[cargo build, cargo test]`.

## Verification

- Freeze gate (cc): `git diff <spec-rev> <tip> -- tests/live_write_guardrail.rs` EMPTY.
- Semantic review (codex): the four seams match ADR 0030; wiring correct (is_live/multiplier/is_mkt/
  notional derivation; gate → posture → connect; combo refuse before gate on real path only; close/cancel/
  preview/paper untouched; gate body unchanged); `shape_preview` JSON unchanged (order-preview tests green).
- Full-suite gate (cc): build + `cargo test` + `cargo clippy --all-targets -- -D warnings`. SAFE with the
  gateway up (guardrail refuses are offline; no live-placing test — the frozen tests are pure).
- Operator live acceptance (cc at merge, `:4001`): `omi --live buy AAPL 100 --limit 250` ⇒ config/exit5,
  no order; `omi --live buy AAPL 1` (MKT) ⇒ config/exit5; `omi --live option-combo …` ⇒ config/exit5;
  `OMI_MAX_NOTIONAL=100000 omi --live buy AAPL 100 --limit 250` ⇒ passes the guardrail. `omi --live
  orders` empty before+after. (The within-cap PASS→place is the operator's first real trial order.)

## Non-goals

True-risk (spread-width) notional; `config.toml` cap key; MKT on options; `modify`/GTC; touching the gate
body, close, cancel, or paper; the trade log (the next feature).
