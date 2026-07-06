# ADR 0030 — live write posture guardrail (LMT-force + notional cap + combo lockout)

Status: accepted · 2026-07-06 · feature: live-write-guardrail

## Context

`require_live_write_gate` (`trade.rs:175`) is the ONLY deterministic write guard: it answers "paper or
live?" (`cfg.port == LIVE_PORT` + `OMI_ALLOW_LIVE=1`). It says nothing about whether the order reaching
live is sane. The order is composed by CC from natural language — there is no deterministic breaker on
magnitude, side, or price. The gate cannot stop "buy 1000 not 100", a fat-fingered strike, or a live MKT
with no price protection. The 2026-07-06 incident (a wrong instruction reached a real order; the gate held
its narrow port-job but had no notion of "should this order exist") is the shape of the risk. Fractional is
not an escape hatch — the gateway refuses `0.1` shares via API (`[10243]`, verified on paper 2026-07-06),
so the minimum live fill is 1 whole share and a notional cap is the only economic breaker. The operator is
about to open a real-money stock+option trial; this guardrail is the blocker before that.

## Decision

Add a **write posture guardrail** ALONGSIDE the port gate (never weakening it), enforced offline
(before connect) and contained in `src/ib/trade.rs`. On the LIVE real-order path it refuses:

1. **not LMT** (D1) — a live opening order with no limit (STK `order_type == "MKT"`) ⇒ refuse. Options/
   combo are already LMT-only, so this bites STK only. Never silently converted to LMT.
2. **over cap** (D2/D3) — `notional = total_quantity × |limit_price| × multiplier` (STK ×1, OPT ×100);
   `notional > cap` ⇒ refuse. `cap` = `OMI_MAX_NOTIONAL` if set, else `DEFAULT_MAX_NOTIONAL = 500.0`.
   A present-but-non-numeric or `≤ 0` / non-finite `OMI_MAX_NOTIONAL` ⇒ refuse (fail-closed, never
   silently fall back to the default).
3. **combo** (D4) — `option-combo` on the live real path ⇒ refuse (combo is paper-only during the trial;
   operationalizes the operator's interlock posture: STK + single-leg live, combo paper).

Scope (D5): the guardrail binds **opening** orders only — `buy`/`sell`/`option-buy`/`option-sell` (they
route through `place_core`) + `option-combo`. `option-close` is EXEMPT (never block an exit; it is
already LMT and routes through `place_with_client` directly, bypassing `place_core`, so the exemption is
structural). `cancel` is N/A (no notional). `--preview` (read-only, ADR 0027) is EXEMPT — it never places
and short-circuits before the guardrail. Paper (`:4002`) is entirely unaffected (MKT + unlimited).

All four refuses use `AppError::config` (code `config`, exit **5**) — uniform with the existing gate, so
CC reads a single "policy refused, fix and retry" signal. The over-cap message names the computed
notional, the cap, and the `OMI_MAX_NOTIONAL` override.

### Seams (pure = FROZEN; wired = review-by-reading)

Pure, FROZEN (tested in `tests/live_write_guardrail.rs`):

```rust
// notional; MKT (limit None) ⇒ None. Mirrors shape_preview (trade.rs:85).
pub fn compute_notional(quantity: f64, limit: Option<f64>, multiplier: f64) -> Option<f64>

// cap resolution; None ⇒ DEFAULT_MAX_NOTIONAL; Some ⇒ parse f64, finite ∧ > 0 else Err. Fail-closed.
pub fn resolve_max_notional(raw: Option<&str>) -> Result<f64, String>

// the posture decision for opening STK/single-leg-option orders.
//   !is_live ⇒ Ok (paper exempt) · is_mkt ⇒ Err(LMT) · notional > cap ⇒ Err(cap) · else Ok
pub fn check_live_write_posture(is_live: bool, is_mkt: bool, notional: Option<f64>, cap: f64) -> Result<(), String>

// combo lockout (tiny; pure so it can be frozen): is_live ⇒ Err(paper-only) · else Ok
pub fn refuse_live_combo_on_live(is_live: bool) -> Result<(), String>
```

Wired (gateway fns, review-by-reading — NOT frozen):

- `place_core` (`trade.rs:468`): after the `cfg.preview` branch, BEFORE/paired with
  `require_live_write_gate` and connect — derive `is_live = cfg.port == LIVE_PORT`, `cap =
  resolve_max_notional(env OMI_MAX_NOTIONAL).map_err(config)`, `multiplier` from
  `contract.security_type` (Option ⇒ 100, else 1), `is_mkt = order.order_type == "MKT"`, `notional =
  compute_notional(order.total_quantity, order.limit_price, multiplier)`; call
  `check_live_write_posture(...).map_err(config)`. Then the existing gate → connect path, unchanged.
- `option_combo` (`trade.rs:713`): on the real path (`!cfg.preview`), before `require_live_write_gate`
  (site `766`), call `refuse_live_combo_on_live(cfg.port == LIVE_PORT).map_err(config)`.

Ordering (both offline, before connect): gate is coarse ("allowed to write live at all?"), posture is
fine ("is this order sane?"). Run the existing `require_live_write_gate` FIRST, then the posture check,
then connect — so a missing `OMI_ALLOW_LIVE` still reports the gate message. (Combo lockout runs before
the gate, so `omi --live option-combo` reports "combo is paper-only" directly.)

## Consequences

- A live opening order is now bounded: LMT-only (price protection) + ≤ $500 default notional (fat-finger
  breaker) + no live combo. The within-cap LMT order proceeds to the unchanged gate → connect → place.
- Every refuse is offline-deterministic (no gateway needed) ⇒ **freezable** (unlike ADR 0029). The frozen
  spec asserts the refuse decisions + notional math + env parse; the within-cap→place path stays operator
  live acceptance (the first trial order) because asserting it would place a real order.
- Paper testing is unchanged (MKT, unlimited) — the trial's zero-risk rehearsal surface is intact.
- `OMI_MAX_NOTIONAL` mirrors `OMI_ALLOW_LIVE`: per-command, auditable, no persistent config needed. A
  typo fails closed (refuse), never widening risk.
- Contained: `require_live_write_gate` untouched; no read command imports the new seams; write code stays
  in `trade.rs`.

## Freeze coverage

FROZEN (`tests/live_write_guardrail.rs`, pure seams): `compute_notional` (LMT value incl. `|limit|`;
MKT⇒None; multiplier 1 vs 100); `resolve_max_notional` (None⇒500.0; "1000"⇒1000.0; ""/"abc"/"0"/"-5"/
"inf"⇒Err); `check_live_write_posture` (paper is_live=false⇒Ok even for huge/MKT; live MKT⇒Err; live
over-cap⇒Err; live within-cap LMT⇒Ok; boundary notional == cap⇒Ok); `refuse_live_combo_on_live`
(live⇒Err, paper⇒Ok).

REVIEW-BY-READING (not frozen — gateway wiring): the `place_core` + `option_combo` wiring (correct
`is_live`/`multiplier`/`is_mkt`/`notional` derivation; runs before connect; maps Err→config; gate
untouched; option-close/cancel/preview/paper paths unchanged).

OPERATOR LIVE ACCEPTANCE (post-merge, the trial itself): the first within-cap live order (1 share)
actually places; an over-cap / MKT / combo live command refuses with exit 5 and NO order (verifiable
with `:4001` down too).

## Alternatives rejected

- **Notional = true risk (spread width / margin).** Fat-finger breaker, not a risk engine; combo (where
  net-limit under-counts risk) is not live in the trial. Revisit only if live combo is opened.
- **Cap in a persistent config file only.** Env mirrors `OMI_ALLOW_LIVE` ergonomics + per-command
  auditability; a `config.toml` key can be added later, env stays primary (env-only this card).
- **Stateful "must --preview before live".** Collapses into the inline notional computation (D2) — no
  statefulness; the mandatory internal `compute_notional` IS the preview's safety core.
- **Silently convert live MKT → LMT / marketable-LMT.** Never mutate the operator's order; refuse
  explicitly and let them restate with a limit.
- **Put the check in `place_with_client` (the universal choke point).** Would catch `option-close` and
  block exits — rejected; the check goes in `place_core` precisely so closes stay exempt.
