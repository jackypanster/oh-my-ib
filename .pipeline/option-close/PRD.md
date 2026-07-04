# PRD — option-close (Phase 2 extension)

Feature: `omi option-close` — close an existing option position **by conid**, LMT-only, TIF=DAY,
paper-first, live double-gated (ADR 0017/0018 apply VERBATIM — zero new safety machinery) — plus
the read-side prerequisite: `positions`/`brief` rows gain security-identity fields so an agent can
tell WHICH conid is the option. Two natural cards: 01 read-side row enrichment, 02 the close verb.
Status: decision-complete (operator /think 2026-07-04: scope "option-close 两张卡" selected +
full-auto authorized; orchestrator calls below flagged for operator override).

## Problem

Closing an option position is the **highest-frequency write action** and today it is manual:
`option-sell` (close long) / `option-buy` (close short) with a re-typed four-tuple. Two
catastrophic failure modes: (a) **identity typo** (one digit of strike/expiry) ⇒ opens a NEW
position instead of closing; (b) **side inversion** (closing a short needs BUY) ⇒ DOUBLES the
position instead of flattening. TWS protocol has no close endpoint (close = opposite-side order;
US options are fungible, no open/close flag) — so this is a convenience+safety layer whose value
is eliminating both failure modes.

Blocking honesty gap: `positions`/`brief` rows carry only `symbol`+`conid`
(`positions.rs::position_row`, 9-key) — an AAPL stock row and an AAPL option row are
**indistinguishable**, though ibapi's portfolio stream already decodes the full identity
(`proto/decoders.rs::decode_contract`: sec_type, expiry, strike, right, multiplier, trading_class).

## Goal

- **Read (card 01)**: `position_row` 9→14 keys — add `sec_type` (always, e.g. "STK"/"OPT") +
  `expiry` (raw YYYYMMDD string) / `strike` (number) / `right` ("C"|"P") / `multiplier`
  (passthrough string), each `null` on non-option rows. `brief` inherits identically (shared seam).
- **Write (card 02)**: `omi option-close --conid N --limit PX [--qty N]` — match the held
  position by conid, **derive side from position sign** (long ⇒ SELL, short ⇒ BUY), default
  qty = |position|, LMT/DAY via the existing placement core.

## Success criteria (acceptance)

1. `omi positions` (paper) emits 14-key rows; STK rows: `sec_type:"STK"` + 4 nulls; OPT rows: all
   populated. `omi brief` positions rows byte-identical shape (shared `position_row`).
2. `omi option-close --conid <held-long-OPT> --limit <px>` exits 0 in the bounded ack window with
   the 10-key ack `{order_id, status, conid, symbol, expiry, strike, right, action:"SELL",
   quantity, limit_price}`; a held SHORT derives `action:"BUY"`.
3. **Anti-open gate**: conid not among current positions ⇒ `not_found` envelope, NO order placed.
   Conid held but not an option ⇒ `usage` naming the actual sec_type and pointing at `sell`.
4. **Anti-double gate**: side is NEVER user-supplied; `--qty` > |position| ⇒ `usage` (over-close
   would flip the position); `--qty` whole ≥ 1.
5. **Wrong-contract gate**: the placement contract is REBUILT from the matched row's decoded
   identity via the proven option builder chain (ADR 0020 D8), then conid-asserted via
   `contract_details` FIRST row == `--conid` (ADR 0021 pattern); mismatch ⇒ `data` error, no order.
6. Validation ordering frozen (offline): usage (`--conid` ≥ 1; `--limit` finite > 0; `--qty`
   finite, whole, ≥ 1) < config (gate) < connection; runtime `not_found`/`data` need a gateway.
7. Live double gate parity (frozen, offline): `--live` without `OMI_ALLOW_LIVE=1` ⇒
   `code="config"` pre-connect; `--port 4001` no-env ⇒ same; paper ungated (dead port ⇒
   `code="connection"`).
8. Bounded ack + no-retry parity: timeout ⇒ exit 6 UNKNOWN-state envelope naming the allocated
   order id + `omi orders`, forbidding blind retry (ADR 0017 verbatim).
9. Containment: ALL write-path code in `src/ib/trade.rs`; card 01 is read-side only
   (`positions.rs`), imports no write symbols; polarity grep unchanged.
10. `cargo build` · clippy `-D warnings` · `cargo test` green; existing frozen suites asserting
    the 9-key row (positions/brief) are re-frozen BY TASK as part of this feature's spec commit.
11. Docs amendment rides the PR: AGENTS.md (full) + CLAUDE.md (short) Phase-2 line adds
    `option-close`; CLAUDE.md stays < 900B (budget computed at arch).
12. **Merge gate (operator/orchestrator, paper `:4002`)**: ensure a filled paper option position
    (acquire via marketable `option-buy` if none) → `positions` shows the OPT row w/ identity →
    `option-close` far-off-market → `orders` shows it working with derived side → `cancel` →
    `option-close` marketable → position flattened (`positions`/`executions`).
    ENVIRONMENTAL: needs a fillable paper option position + gateway Read-Only API off.

## Scope

- `src/ib/positions.rs`: `position_row` 14-key (brief parity free via shared seam).
- `src/cli.rs`: `OptionClose(OptionCloseArgs { conid: i32, limit: f64, qty: Option<f64> })`.
- `src/ib/trade.rs` (the ONLY write module): pure seams — close derivation (position qty →
  side + default qty) + `shape_option_close_ack` (10-key); gateway fn `option_close`
  **single-connect** (gate → connect → `account_updates` drain matching conid → rebuild contract
  from decoded fields via builder chain → `contract_details` conid assert → `place_with_client`).
- `src/ib/mod.rs` re-export + `src/main.rs` dispatch arm.
- AGENTS.md + CLAUDE.md amendment (criterion 11). No new dependency.

## Non-scope (explicitly NOT this feature)

- No MKT/GTC/stops/modify; no auto-pricing (agent prices via `option-quote` first).
- No combo/BAG whole-structure close (legs close individually via this verb); no exercise.
- No generic STK close (`sell` covers it); no close-all sweep; no non-USD.

## Resolved decisions (locked)

- D1 **Conid-addressed close, not four-tuple re-entry** — the position match IS the anti-open
  gate; a conid that isn't held cannot place anything.
- D2 **Side/qty derived from the held position** (long→SELL, short→BUY; default full close;
  over-close rejected). Side is not an argument.
- D3 **LMT-only, `--limit` required** (option-orders D2 parity; operator may override).
- D4 **Rebuild + conid-assert placement path**: builder chain from portfolio-decoded identity
  (symbol/expiry/strike/right/trading_class, SMART + row currency) + `contract_details` first-row
  conid assert. REJECTED: resubmitting the portfolio-decoded Contract verbatim (unverified on the
  Tiger gateway; assert converts any drift into a clean pre-order failure).
- D5 **Flat row enrichment with nulls** (stable keys; `limit_price: null` precedent) on the shared
  `position_row` seam — additive keys, existing 9 keys untouched (backward-compatible).
- D6 **Single-connect invariant**: one client for drain + resolve + place (option-combo review
  lesson — a second same-client-id connect wedges the gateway).
- D7 **10-key ack** echoing the RESOLVED identity + derived action (not user echo — there is no
  user four-tuple to echo).
- D8 **Safety parity, zero new machinery**: `require_live_write_gate`, local order-id allocator,
  `TAKE_FIRST_TIMEOUT` bounded first-ack, no-retry — all VERBATIM.

## Risks / fragile assumptions

- **Tiger portfolio row content unverified** (expiry format, trading_class presence, multiplier):
  card 01 acceptance OBSERVES the actual row; the rebuild path consumes those fields, and the D4
  conid assert turns any mismatch into a pre-order `data` failure — never a wrong order. Fallback
  (documented, needs operator ack): resubmit decoded contract with exchange forced SMART.
- **Frozen-spec ripple**: positions/brief shape suites re-freeze (task owns, one spec commit);
  CLAUDE.md 900B budget computed at arch (option-orders lesson — arch text overran the budget).
- **Drain inside a write command**: `account_updates` drain is End-marker-bounded (verified
  pattern in `positions`); worst case = existing timeout envelope.
- Paper acceptance needs a REAL filled option position (environmental, criterion 12).
- Rollback: additive verb + additive row keys; one revert removes the surface.

## Verification

- Offline frozen spec (task freezes, one spec commit, two cards): card 01 — `position_row`
  14-key exact shape + null semantics on synthetic `AccountPortfolioValue` (STK vs OPT), brief
  parity; card 02 — pure seams (derivation: long/short/default-qty/over-close; 10-key ack),
  validation matrix (conid 0|-1, limit missing|0|inf|NaN, qty 0|1.5|inf ⇒ usage, pre-connect),
  gate matrix parity (no-env config / effective-port / paper-dead connection), `--help` lists the
  verb. Deliberate gate-pass omission (live-order hazard — option-orders precedent).
- Review-by-reading: containment grep, single-connect, assert-before-place ordering.
- Live (paper `:4002`): criterion 12 lifecycle.
