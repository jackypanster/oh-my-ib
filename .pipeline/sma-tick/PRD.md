# PRD — sma-tick

Stage: prd · feature: sma-tick · repo: jackypanster/oh-my-ib · branch: main
Author: cc. Provenance: the read-only `sma-signal` (PR #31) surfaces the Faber 200-day month-end
HOLD/EXIT signal; `sma-tick` is the deliberately-deferred **active** counterpart that ACTS on it.
Four decisions locked via /think AskUserQuestion: binary target position, QQQ-only, lot=10, paper-only.

## Problem

`sma-signal` tells the operator HOLD/EXIT; execution is manual. The operator now wants the
**automatic** version: each month, make the actual QQQ position MATCH the signal — enter when it says
HOLD, exit to cash when it says EXIT — without hand-placing orders. This is a WRITE feature (real
orders), so it inherits the full write-safety architecture (ADR 0017): containment, paper-first,
gated. It is the same shape as `grid-tick` but with a binary signal-driven target instead of a grid.

## Goal

Add `omi sma-tick [SYMBOL] --lot 10`: compute the 200-day month-end signal for the symbol, read the
current position, and **reconcile the position to the target** — HOLD ⇒ target = `lot` shares, EXIT ⇒
target = 0 — by placing one buy/sell for the difference. **Paper-only in v1** (`:4002`). `--dry-run`
shows the intended action without placing.

## Decisions (provenance-tagged; ✅ = human-confirmed /think AskUserQuestion)

- **D-SEMANTICS — binary target position, NOT a ladder.** ✅. HOLD ⇒ hold exactly `lot` shares; EXIT ⇒
  hold 0. Enter = buy up to `lot`; exit = sell the whole position. This is the strategy strategy-lab
  actually validated (all-in / all-out timing, at a fixed size). NOT "buy 10 more each month"
  (an untested accumulation strategy — explicitly rejected).
- **D-SCOPE — QQQ, `lot` default 10, single symbol, flags not a config file.** ✅ (operator: "只针对
  QQQ,每次10"). `omi sma-tick [SYMBOL] --lot <n> --sma <n>`; SYMBOL positional (default QQQ). A single
  symbol needs no toml (unlike grid-tick's multi-symbol config) — keep it minimal.
- **D-TARGET — paper `:4002` ONLY in v1.** ✅. 10 QQQ ≈ $7,200 ≫ the $500 live notional cap, which
  would refuse it (same wall as grid-tick). `sma_tick_cmd` hard-refuses `cfg.port == LIVE_PORT` (config
  error, offline). Live is a separate future decision (raise `OMI_MAX_NOTIONAL`, neutering the cap).
- **D-PURE-SEAM — a pure `plan_sma_tick(state, current_qty, lot) -> TickAction` is the frozen heart.** ✅
  (design, mirrors grid-tick). The reconcile decision is pure/offline. The gateway (compute signal, read
  position, place the order) is review-by-reading.
- **D-REUSE — compose existing seams; add almost no new write code.** ✅ (code-survey). Signal:
  `sma_signal` (pure, PR #31). Order build: `build_stk_order` (unchanged). Placement: `place_with_client`
  (pub(crate), from grid-tick). Position read: `positions()`. Live guard: `LIVE_PORT`. No raw
  `place_order` in the new module ⇒ ADR 0017 containment holds (sma-tick is a sanctioned consumer of the
  trade.rs choke points, like grid-tick).
- **D-ORDER — order type is an arch decision; recommend MKT DAY (fills at next open, matching Faber's
  "act at the open"), accepting the [399] "queued to next RTH open" as a SUBMITTED success (not an
  error).** ✅-ish (design; arch confirms). Alternative: a marketable LMT at the latest close ± buffer.
  On paper the notional cap / live-LMT rule don't apply, so MKT is admissible. The pure plan is
  order-type-agnostic (it only decides buy/sell/qty).

## The pure planner contract (this is what `pipeline-task` freezes)

```
TickAction = Buy{ qty: f64 } | Sell{ qty: f64 } | Noop

plan_sma_tick(state: SignalState, current_qty: f64, lot: f64) -> TickAction

rule:
  target = match state { Hold => lot, Exit => 0.0, Insufficient => (return Noop) }   // no data ⇒ don't trade
  delta  = target - current_qty
  if delta >  1e-9 → Buy{ delta }
  if delta < -1e-9 → Sell{ -delta }
  else             → Noop
```

`SignalState` is the existing enum (`oh_my_ib::ib::SignalState`, PR #31). Reconciles from ANY current
qty (0 → buy to lot; lot → noop; >lot → sell down; short → buy to cover then to target — all correct by
the same delta math). Offline, deterministic ⇒ the whole freeze surface.

## Scope

- **IN** `src/ib/sma_tick.rs` (NEW): pure `plan_sma_tick` + `TickAction` + gateway `sma_tick_cmd(cfg,
  args)` (refuse live → connect → compute `sma_signal` for the symbol (reuse the bar fetch) → read
  current qty via `positions()` → `plan_sma_tick` → execute via `build_stk_order` + `place_with_client`,
  unless `--dry-run`) → JSON `{symbol, signal, as_of, current_qty, target_qty, action, order?}`.
- **IN** `src/cli.rs`: `SmaTick(SmaTickArgs)` + `struct SmaTickArgs { symbol: Option<String> (default
  QQQ), #[arg(long, default_value_t=10.0)] lot: f64, #[arg(long, default_value_t=200)] sma: usize,
  #[arg(long)] dry_run: bool }`.
- **IN** `src/main.rs` dispatch + `src/ib/mod.rs` re-export (`plan_sma_tick`, `sma_tick_cmd`, `TickAction`).
- **IN** NEW frozen spec `tests/sma_tick.rs`.
- **OUT** (non-scope): live path (D-TARGET); multi-symbol / toml config (single-symbol flags, D-SCOPE);
  the laddering strategy (D-SEMANTICS); resting-order/cancel management (one MKT per tick reconciles;
  no book to maintain); monthly scheduling + Telegram (ops glue — extend `sma-monthly` later); other
  order types beyond the arch-chosen one; any change to the existing gates / notional cap / grid-tick /
  sma-signal.

## Success criteria (acceptance)

1. **Pure `plan_sma_tick` (offline, FROZEN):** Hold+current 0+lot 10 ⇒ Buy 10; Hold+current 10 ⇒ Noop;
   Hold+current 4 ⇒ Buy 6; Hold+current 15 ⇒ Sell 5 (reconcile down); Exit+current 10 ⇒ Sell 10;
   Exit+current 0 ⇒ Noop; Insufficient ⇒ Noop. [frozen]
2. **Paper-only guard:** `omi --live sma-tick QQQ` ⇒ `code="config"` "paper-only", offline. [frozen/read]
3. **CLI (operator, paper `:4002`):** `omi sma-tick QQQ --lot 10 --dry-run` prints the current signal +
   current/target qty + intended action, no order. A real `omi sma-tick QQQ --lot 10` places the
   reconcile order (buy to 10 when HOLD & flat; sell to 0 when EXIT & holding); `omi orders`/`positions`
   reflect it. [operator paper]
4. `cargo build` · full `cargo test` · `cargo clippy --all-targets -- -D warnings` green; all prior
   suites byte-identical. [verify]

## Gotchas

- **Binary target, not a ladder** (D-SEMANTICS) — target is `lot` or `0`, never accumulating.
- **Paper-only** — hard-refuse `cfg.port == LIVE_PORT` BEFORE connect (like grid-tick). 10 QQQ ≫ $500 cap.
- **Compose the trade.rs choke points** — `build_stk_order` + `place_with_client`; NO raw `place_order`
  in `sma_tick.rs` (ADR 0017; review greps).
- **MKT queued-to-open ([399])** — when the market is closed a MKT queues to next RTH open; treat that
  as a SUBMITTED success, not a failure (arch/impl decide the exact handling; it matches Faber's
  act-at-open). Or use a marketable LMT.
- **Reuse `sma_signal`** (pure) for the signal; the gateway fetches 2Y Day bars like `sma_signal_cmd`
  (arch: expose a shared fetch or re-do the `historical_data` call).
- **`current_qty` from `positions()`** — extract the symbol's qty (0 if absent).

## Verify

`cargo build` · `cargo test` (new `tests/sma_tick.rs` red→green; all prior suites green) · `cargo clippy
--all-targets -- -D warnings`. Operator paper: criterion 3 on `:4002`.

## For arch (next stage)

1. Module placement: new WRITE-orchestration module `src/ib/sma_tick.rs` (pure `plan_sma_tick` + gateway).
   Author ADR 0035 (active 200SMA timing executor; binary target; paper-only; MKT-vs-LMT order choice;
   reuses sma_signal + grid-tick's place_with_client; Phase-2-of-sma-signal provenance).
2. Resolve the ORDER TYPE (MKT accept-[399]-as-submitted vs marketable LMT) + how the gateway maps a
   `TickAction` onto `build_stk_order` + `place_with_client` (side, qty, price?).
3. Decide how the gateway computes the signal (share a bar-fetch helper with `sma_signal_cmd`, or re-call
   `historical_data`); pin `plan_sma_tick`/`TickAction` for the freeze.
4. Confirm the JSON output shape + `--dry-run`; CONTEXT.md glossary ("target position", "reconcile tick").
