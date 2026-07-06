# PRD — live-write-guardrail

Stage: prd · feature: live-write-guardrail · repo: jackypanster/oh-my-ib · branch: main
Author: cc. Live-trial BLOCKER (operator opens real-money stock+option trading only after this lands).
Decisions locked with operator 2026-07-06 (three questions); arch to confirm + ADR 0030.

## Problem

`omi`'s ONLY deterministic write guard is the paper-vs-live gate (`require_live_write_gate`,
`trade.rs:175`): live (`:4001`) needs `--live` + `OMI_ALLOW_LIVE=1`; paper (`:4002`) is ungated. That
guard answers "paper or live?" — it says NOTHING about whether the order that reaches live is sane. In
this product the order is composed by CC translating natural language, with **zero** deterministic guard
on magnitude, side, or price. The gate cannot stop "buy 1000 not 100", a fat-fingered strike, or a live
MKT with no price protection. The 2026-07-06 safety incident (a wrong instruction — a test — reached a
REAL order; gate held its narrow port-job but had no notion of "should this order exist") is the shape of
the risk. The robustness frontier has moved ABOVE the CLI, to the CC↔natural-language layer, which has no
circuit-breaker.

Fractional is NOT an escape hatch: verified 2026-07-06 on paper (`DUQ653733`), a `0.1`-share order is
refused by the gateway — `[10243] Fractional-sized order cannot be placed via API` (exit 4). So the
minimum real live fill is **1 whole share**, and an economic notional cap is the only breaker between CC
and a live account.

## Goal

On LIVE (`:4001`) opening orders, refuse — BEFORE any connection, offline-deterministic, contained in
`src/ib/trade.rs` — any order that (a) is not LMT, (b) exceeds a configurable notional cap, or (c) is a
combo. Paper (`:4002`) stays completely unaffected (MKT + unlimited, for zero-risk testing). The refuse
paths never connect and never place, so they are freezable as pure/offline tests (unlike the prior
can't-freeze live-gate fix).

## Decisions (locked with operator; arch to record in ADR 0030)

- **D1 — live orders must be LMT.** A live opening order with no limit (STK MKT: `OrderArgs.limit ==
  None` ⇒ `order_type == "MKT"`) is REFUSED (config/usage error: "live orders must be LMT — pass
  --limit"). Never silently converted. Options/combo are already LMT-only, so this bites STK only. MKT
  stays allowed on paper.
- **D2 — notional cap.** `notional = total_quantity × |limit_price| × multiplier` (STK ×1, OPT ×100).
  On live, `notional > cap` ⇒ REFUSED (config error naming the computed notional, the cap, and how to
  raise it). Reuses the exact math already in `shape_preview` (`trade.rs:85`) — extract to a pure seam.
- **D3 — cap default $500, `OMI_MAX_NOTIONAL` overrides.** Absent ⇒ $500. Present + non-numeric or ≤0 ⇒
  config error (fail-closed, never silently default). Env matches `OMI_ALLOW_LIVE` ergonomics and keeps
  the value per-command auditable. A `config.toml` key is OPTIONAL (arch decides; env is the primary).
  $500 admits a 1-share live test of most mega-caps (AAPL≈$250) and blocks fat-fingers (100 AAPL ≈ $25k).
- **D4 — live combo refused.** `option-combo` on live ⇒ REFUSED (config error: combo is paper-only during
  the trial). Operationalizes the operator's interlock posture (STK + single-leg live; combo paper).
- **D5 — scope of the cap: OPENING orders only; never block an exit.** The LMT-force + cap apply to
  `buy`/`sell`/`option-buy`/`option-sell` (they route through `place_core`, `trade.rs:468`).
  `option-close` is EXEMPT (risk-reducing — never block an exit; it is already LMT and calls
  `place_with_client` directly, bypassing `place_core`, so exemption is structural). `cancel` is N/A (no
  notional; removes an order). `--preview` (read-only, ADR 0027) is EXEMPT — it never places.

## Success criteria

1. `omi --live buy AAPL 100 --limit 250` (notional $25 000 > $500) ⇒ refused with a `config` error naming
   notional + cap, BEFORE connect (works with `:4001` down too). No order. [offline / frozen + operator]
2. `omi --live buy AAPL 1` (MKT, no --limit) ⇒ refused ("live orders must be LMT"), before connect. [offline]
3. `OMI_MAX_NOTIONAL=100000 omi --live buy AAPL 100 --limit 250` ⇒ passes the guardrail (proceeds to the
   existing gate/connect). [offline logic] · `OMI_MAX_NOTIONAL=abc` ⇒ config error (fail-closed). [offline]
4. `omi --live option-combo --action BUY ...` ⇒ refused (combo paper-only), before connect. [offline]
5. `omi --live option-buy ... --limit 3 --qty 1` (notional $300 < $500) ⇒ passes the guardrail. [logic]
   The within-cap PASS→place path is NOT unit-tested (would place a real order); it is proven by the
   operator's first live trial order (1 share within cap) after merge.
6. Paper unaffected: `omi buy AAPL 100` (MKT, no cap) still places on paper; `omi --preview --live buy
   AAPL 100 --limit 250` still previews (read-only, no refuse). [operator paper + read]
7. Contained: diff is `src/ib/trade.rs` (+ `cli.rs` help, + optionally `config.rs` if the env read lands
   there) + the new frozen test file. `require_live_write_gate` semantics preserved (extended alongside,
   never weakened). No read command imports the new seams. [read / freeze gate]
8. `cargo build` · full `cargo test` · `cargo clippy --all-targets -- -D warnings` all green.

## Scope

- IN: `src/ib/trade.rs` — a pure `compute_notional` seam (extract from `shape_preview`), a pure
  live-posture decision seam (given effective-live? + is-MKT + notional + cap → Ok / refuse-reason), a
  thin env reader for `OMI_MAX_NOTIONAL`, and wiring the posture check into `place_core` (buy/sell/
  option-buy/option-sell) + a live-refuse line in `option_combo`. `src/cli.rs` help text. NEW frozen
  spec file `tests/live_write_guardrail.rs`.
- OUT: paper behavior; the trade log (the NEXT feature); `modify`/GTC/option-MKT; combo true-risk (width)
  modeling (combo not live in the trial); fractional (API-dead, 10243); `option-close` cap (exempt) and
  `cancel` (N/A); weakening/refactoring `require_live_write_gate`.

## Non-scope / rejected alternatives (for arch to record in the ADR)

- **Notional = true risk (spread width, margin).** Rejected for the trial: this is a fat-finger breaker,
  not a risk engine; combo (the case where net-limit under-counts risk) is not live. Revisit if live
  combo is ever opened.
- **Cap in a persistent config file only (no env).** Rejected — env mirrors `OMI_ALLOW_LIVE` ergonomics
  and is auditable per-command; a config.toml key may be added as a convenience, env stays primary.
- **Force preview as a stateful two-step (must `--preview` before live).** Rejected — collapses into the
  inline notional check (D2); no statefulness, the "preview" IS the mandatory internal computation.
- **Silently convert live MKT → LMT (or marketable-LMT).** Rejected — never mutate the operator's order;
  refuse explicitly and let them restate with a limit.

## Freeze / spec note

NEW feature ⇒ NEW frozen spec file `tests/live_write_guardrail.rs` (touches no other feature's frozen
spec — no re-freeze). Normal freeze ceremony (one spec-rev). Because every REFUSE path is offline-
deterministic (decided before connect), it IS freezable — the frozen red asserts: `compute_notional`
values (pure); the posture decision refuses live MKT / over-cap / combo and passes within-cap LMT; the
`OMI_MAX_NOTIONAL` parse (absent⇒$500, bad⇒error). The within-cap→actually-place path is NOT frozen (it
would place a real live order) — it is operator live acceptance (the first trial order), like every prior
gateway-dependent behavior. arch/task pin the exact frozen assertions; env-reading seams follow the
`require_live_write_gate` precedent (thin env wrapper over a pure, cap-parameterized decision fn).

## Gotchas

- The guard fires ONLY when `cfg.port == LIVE_PORT` AND `!cfg.preview`. Paper and preview are exempt by
  construction. Mirror the gate's port key exactly (`--live` and hand-set `--port 4001` both count).
- MKT detection: STK `OrderArgs.limit == None` ⇒ `build_stk_order` sets `order_type = "MKT"`
  (`trade.rs:44`). Options/combo are always LMT (`build_option_order`/`build_combo_order`), so D1 only
  bites STK; the cap (D2) still applies to all opening verbs.
- Multiplier by security type: read `contract.security_type` at the `place_core` seam (STK ×1, OPT ×100).
- `option-close` routes through `place_with_client` directly (NOT `place_core`) → structurally exempt
  from the cap (never block an exit). Do NOT add the check to `place_with_client` (would catch closes).
- Fail-closed on a bad `OMI_MAX_NOTIONAL` — a typo must refuse, never fall back to $500 silently.
- Do NOT run the full `cargo test` while the Tiger gateway is UP unless the live-gate guard (`5b5b59b`)
  is present on the branch — it is on trunk now, so branches cut from trunk inherit it.

## Verify

`cargo build` · `cargo test` · `cargo clippy --all-targets -- -D warnings` · operator paper: `omi buy
AAPL 100` still places on paper (guard exempt); offline refuse checks need NO gateway. Live-pass is the
operator's first trial order post-merge.
