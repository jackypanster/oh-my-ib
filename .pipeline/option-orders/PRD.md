# PRD — option-orders (Phase 2 step 3)

Feature: `omi option-buy` / `omi option-sell` — single-leg US equity option order placement,
**LMT-only, TIF=DAY**, paper-first, live double-gated. Extends the Phase-2 write path
(ADR 0017/0018 apply VERBATIM — zero new safety machinery). `omi cancel` is REUSED as-is
(cancel_order is order-id-based, sec-type-agnostic — no new cancel verb).
Status: decision-complete (operator /think 2026-07-04 with standing full-auto authority:
"一路自动继续…最后merge之前才通知我"; D2 LMT-only is the orchestrator's call under that
authority, flagged for operator override).

## Problem

Phase 2 ladder: ~~1. STK orders~~ ✅ → ~~2. options data read~~ ✅ → **3. single-leg option
orders** → 4. combos (BAG/spread). The agent can now discover (`option-chain`) and price
(`option-quote`) an option contract but cannot act on it. Step 3 closes the loop for
single-leg strategies (covered calls, cash-secured puts, directional longs).

## Goal

Two write verbs on the existing placement core:

- `omi option-buy  --symbol S --expiry YYYYMMDD --strike N --right C|P --qty N --limit PX
  [--trading-class TC --exchange SMART --currency USD]` — BUY a single-leg option, LMT, DAY.
- `omi option-sell …` — mirror (SELL; covered/naked distinction is the broker's margin
  problem, not the CLI's).
- Cancel: existing `omi cancel ORDER_ID` (documented, tested by the existing frozen suite).

## Success criteria (acceptance)

1. `omi option-buy --symbol AAPL --expiry <near-month> --strike <ATM> --right C --qty 1
   --limit <far-below-market>` (paper `:4002` default) exits 0 within the bounded ack window
   printing the 9-key option ack: `{order_id, status, symbol, expiry, strike, right, action,
   quantity, limit_price}` (`status` = first gateway ack, e.g. PreSubmitted/Submitted).
2. `omi option-sell` mirrors (action SELL).
3. **LMT-only**: `--limit` is REQUIRED (missing ⇒ usage envelope, offline). No MKT path
   exists (deliberate v1 boundary, D2).
4. **Live double gate parity (frozen, offline)**: `--live` without `OMI_ALLOW_LIVE=1` ⇒
   `code="config"` BEFORE any connection, both verbs; `--port 4001` no-env ⇒ same
   (effective-port rule); env+live+dead-port ⇒ `code="connection"`; paper dead-port ⇒
   `code="connection"` (ungated).
5. **Local validation (usage, offline, precedes gate+connect)**: `--qty` finite, ≥ 1, whole
   number (options trade in whole contracts); `--limit` finite > 0; `--right` ∈
   C|CALL|P|PUT case-insensitive; `--expiry` 8-digit YYYYMMDD (m 1-12, d 1-31); `--strike`
   finite > 0. ALL numeric args NaN/inf-rejected (review-01 lesson, applied proactively).
6. Bounded ack + no-retry parity: timeout ⇒ exit 6 UNKNOWN-state envelope naming the
   allocated order id + `omi orders`, forbidding blind retry (ADR 0017 §3-4 verbatim).
7. Write containment: ALL new write-path code in `src/ib/trade.rs`; polarity grep unchanged
   (write symbols nowhere else). Read commands byte-identical; existing frozen suite green.
8. `cargo build` · clippy `-D warnings` · `cargo test` green (whole suite, post-impl).
9. **Docs amendment rides the PR**: AGENTS.md + CLAUDE.md Phase-2 line updated (option
   ORDERS now exist, single-leg LMT/DAY, same gates; combos still out).
10. **Merge gate (operator, paper `:4002`, PRD criterion — analogue of stk criterion 11)**:
    far-below-market LMT `option-buy` → `omi orders` shows it working → `omi cancel <id>` →
    `omi completed-orders` shows Cancelled → `omi positions` unchanged.
    ENVIRONMENTAL UNKNOWN: the paper account's options-trading permission is unverified —
    a permission rejection at acceptance is an OBSERVATION (journal + operator decision),
    not an impl failure.

## Scope

- `src/cli.rs`: `OptionBuy(OptionOrderArgs)`, `OptionSell(OptionOrderArgs)`;
  `OptionOrderArgs` (all-flags; `--limit` required; exact struct pinned by arch).
- `src/ib/trade.rs` (the ONLY write module): pure seams `build_option_order` (contract via
  the option-quote builder chain + LMT/DAY Order) + `shape_option_order_ack` (9-key);
  gateway fns `option_buy`/`option_sell` on a shared placement core extracted from the
  existing `place` (stk behavior byte-identical — the frozen stk suite is the regression
  net, D7).
- `src/ib/option_quote.rs`: **visibility-only** — `normalize_right`/`parse_expiry` →
  `pub(crate)` for reuse by trade.rs validation (D6). No behavior change.
- `src/ib/mod.rs` re-exports + `src/main.rs` two dispatch arms.
- AGENTS.md + CLAUDE.md amendment (criterion 9).
- No new dependency.

## Non-scope (explicitly NOT this feature)

- **No MKT option orders** (D2, deliberate: structurally wide spreads; marketable LMT covers
  the need; additive v1.1 candidate).
- No combos/BAG/spreads (step 4), no modify (cancel + re-place), no GTC/stops/brackets/OCA,
- No exercise/assignment, no FOP (futures options), no non-USD.
- No new cancel verb; no notional cap; no dry-run (parity with stk-orders operator D3/D5).

## Resolved decisions (locked)

- D1 **New verbs, cancel reused** — `buy`/`sell` stay STK-pure (highest-stakes path clarity;
  stk-orders D2 verb-per-intent precedent); extending them with option flags rejected.
- D2 **LMT-only v1** (orchestrator call under standing authority, operator may override):
  `--limit` required; no MKT arm in `build_option_order`.
- D3 **Safety parity, zero new machinery**: `require_live_write_gate`, local order-id
  allocator (ADR 0018), `TAKE_FIRST_TIMEOUT` bounded first-ack, no-retry — all VERBATIM.
- D4 **Whole-contract quantity**: finite ∧ ≥1 ∧ fract()==0, else usage. Finite checks on
  every numeric arg (strike/limit/qty).
- D5 **9-key option ack**, stk 6-key ack untouched (both frozen, disjoint).
- D6 **Validation reuse via pub(crate)** promotion of option_quote's `normalize_right` +
  `parse_expiry` (read-module helpers into the write module is a one-way, read-safe import).
- D7 **Shared placement core**: refactor `place` into a contract-agnostic core; stk frozen
  suite guards byte-identity.
- D8 **Contract build = option-quote's proven builder chain** (`Contract::call/put`
  `.strike` `.expires_on` `.on_exchange` `.in_currency` [`.trading_class`] `.build()` —
  SMART/USD/×100 defaults; live-accepted by the gateway during options-read acceptance).

## Risks / fragile assumptions

- **Paper options permission unknown** (criterion 10): a broker-side rejection surfaces as a
  clean `data`/order-status envelope — journaled observation, operator decides. Not an impl
  defect.
- **place-core refactor touches the live stk path**: mitigated by the frozen stk suite
  (byte-identical acks asserted) + review-by-reading.
- MKT absence may annoy an aggressive agent: deliberate; marketable LMT is the documented
  pattern (CONTEXT.md).
- Wedge dossier: order-id channel already proven on this gateway (stk acceptance); every
  wait bounded; worst case = UNKNOWN envelope.
- Rollback: additive verbs + contained trade.rs changes; one revert removes the surface
  (docs amendment reverts with it).

## Verification

- Offline frozen spec (`tests/option_orders_command.rs`, one card): `build_option_order`
  exact Contract/Order fields (call+put × buy/sell, trading_class, LMT/DAY);
  `shape_option_order_ack` 9-key; gate matrix (2 verbs × no-env config / effective-port /
  env+dead connection / paper-dead connection); validation matrix (qty 0|-1|1.5|inf, limit
  missing|0|inf, right X, expiry malformed, strike inf ⇒ usage, all pre-connect); `--help`
  lists both verbs; existing suite untouched green.
- Review-by-reading: shared core refactor (stk byte-identity), containment grep, docs
  amendment verbatim.
- Live (operator, paper `:4002`): criterion 10 lifecycle.
