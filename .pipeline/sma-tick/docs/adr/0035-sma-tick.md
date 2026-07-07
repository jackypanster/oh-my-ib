# ADR 0035 — sma-tick (active 200-day month-end timing executor)

Status: accepted · 2026-07-07 · feature: sma-tick · extends ADR 0017 (write containment) + ADR 0033
(write-orchestration pattern) + ADR 0034 (reuses the sma-signal seam). Paper-only v1.

## Context

`sma-signal` (ADR 0034, PR #31) surfaces the Faber 200-day month-end HOLD/EXIT signal read-only; the
operator executes by hand. The operator now wants the **active** version: monthly, make the actual QQQ
position match the signal automatically. This is a WRITE feature, so it inherits ADR 0017 (containment,
paper-first, gated). It is the `grid-tick` shape (ADR 0033) with a binary signal-driven target.

## Decision — `omi sma-tick [SYMBOL] --lot 10`: reconcile the position to the signal's binary target

A pure `plan_sma_tick(state, current_qty, lot) -> TickAction` (frozen) decides Buy/Sell/Noop; a thin
gateway computes the signal (reusing `sma_signal`), reads the position, and places one order via the
existing trade.rs choke points. Paper-only.

### D-SEMANTICS — binary target, not a ladder

HOLD ⇒ target = `lot` shares; EXIT ⇒ target = 0; INSUFFICIENT ⇒ no trade. `delta = target - current_qty`
→ Buy(delta) / Sell(-delta) / Noop. This is the all-in/all-out timing strategy strategy-lab validated,
at a fixed size. Rejected: "buy `lot` more each month" (unbounded accumulation, an untested strategy).
The delta math reconciles from any current qty (0→buy to lot; lot→noop; >lot→sell down).

### D-SCOPE — QQQ, lot default 10, single symbol, flags (no toml)

`omi sma-tick [SYMBOL=QQQ] --lot 10 --sma 200 --dry-run`. One symbol needs no config file (unlike
grid-tick's multi-symbol toml) — keep it minimal.

### D-TARGET — paper `:4002` only in v1

`sma_tick_cmd` hard-refuses `cfg.port == LIVE_PORT` (config error, offline). 10 QQQ ≈ $7,200 ≫ the $500
`OMI_MAX_NOTIONAL` cap, which would refuse it on live (same wall as grid-tick). Live = a separate future
decision (raise the cap, neutering the fat-finger guard). Because it can't reach live, the double gate /
notional machinery is never exercised and stays untouched.

### D-ORDER — marketable LMT, NOT MKT

A MKT order placed outside RTH returns `[399] … will not be placed until 09:30`, which the place path
surfaces as an error — sma-tick would fail whenever it runs while the market is closed (i.e. most of the
time). A **marketable LMT** (Buy at `round2(latest_close * 1.02)`, Sell at `round2(latest_close * 0.98)`)
fills at the next open like a MKT but rests cleanly as `PreSubmitted` when closed (no [399]), reuses
`place_with_client` UNCHANGED, and is live-shaped (live requires LMT). `latest_close` is already on the
computed `SmaSignal` — no extra fetch. Fixed 2% buffer in v1; a gap beyond it self-heals next tick.

### D-CONTAINMENT — composes the trade.rs choke points (ADR 0017 holds)

`sma_tick.rs` contains NO raw `place_order`/`cancel_order`; it builds via `build_stk_order` and places via
`place_with_client` (pub(crate), from grid-tick) — a sanctioned choke-point consumer, like grid-tick and
place_core. The signal reuses `sma_signal`; a `pub(crate) signal_for` helper is extracted from
`sma_signal_cmd` so both share the fetch (sma-signal's frozen behavior + JSON stay byte-identical).

## Consequences

- `omi sma-tick QQQ --lot 10` on `:4002`: HOLD & flat ⇒ buys to 10; EXIT & holding ⇒ sells to 0; already
  at target ⇒ no-op. `--dry-run` shows signal + current/target + intended action, no order. Monthly
  scheduling + a Telegram confirmation are ops glue (extend `sma-monthly`), not this feature.
- New crate surface: `sma_tick` module (`plan_sma_tick`/`sma_tick_cmd`/`TickAction`); a `pub(crate)
  signal_for` in signal.rs. No existing behavior changes; grid-tick / sma-signal / gates untouched.
- Real (paper) money moves automatically — the reason it was split from the read-only signal and gated
  paper-first. Live promotion is a deliberate later step.

## Freeze coverage

- **FROZEN** (`tests/sma_tick.rs`): `plan_sma_tick` — Hold+0⇒Buy lot; Hold+lot⇒Noop; Hold+partial⇒Buy diff;
  Hold+over⇒Sell down; Exit+held⇒Sell all; Exit+flat⇒Noop; Insufficient⇒Noop.
- **REVIEW-BY-READING**: the gateway (paper-only guard; `signal_for` reuse; position read; marketable-LMT
  price; `build_stk_order`+`place_with_client` execution; `--dry-run`; JSON), the containment grep (no raw
  place_order), the `signal_for` extraction leaving sma-signal byte-identical.
- **OPERATOR ACCEPTANCE** (paper `:4002`): `omi sma-tick QQQ --dry-run` shows the plan; a real run places
  the reconcile order; `omi orders`/`positions` reflect it; `omi --live sma-tick` refused paper-only.

## Alternatives rejected

- **MKT order** — [399] error outside RTH (D-ORDER).
- **Ladder ±lot/month** — unbounded accumulation, untested (D-SEMANTICS).
- **Live in v1** — $500 cap refuses 10 QQQ; deferred (D-TARGET).
- **Multi-symbol toml** — one symbol needs no config (D-SCOPE).
- **Re-implement the write/signal** — reuse `place_with_client` + `sma_signal` (D-CONTAINMENT).
