# PRD — options-read (Phase 2 step 2)

Feature: `omi option-chain` + `omi option-quote` — READ-ONLY options data: chain discovery
(expirations × strikes per exchange/trading-class) and single-contract snapshot quote with
best-effort greeks. Zero writes; `src/ib/trade.rs` untouched; the write gates are unaffected.
Status: decision-complete (operator ran `/think` interactively 2026-07-04 and approved
("同意,开干"); the ibapi read surface was verified in local crate source, exact call shapes
are arch's to pin).

## Problem

Phase 2 ladder: ~~1. STK orders~~ ✅ → **2. options data read** → 3. single-leg option orders
→ 4. combos (BAG/spread). The agent cannot order what it cannot see: `omi` has zero options
visibility — no way to discover which option contracts exist for an underlying (expirations,
strikes, trading classes, multiplier) nor to price a specific contract (bid/ask/last +
greeks). Step 2 closes the data gap so step 3's write path has a discovery + pricing
substrate.

## Goal

Two read-only commands on the existing connect-per-command model:

- `omi option-chain SYMBOL [--exchange SMART]` — resolve the underlying's conid
  (contract_details), then reqSecDefOptParams; emit the underlying + one row per
  (exchange, trading_class): multiplier, expirations[], strikes[].
- `omi option-quote --symbol S --expiry YYYYMMDD --strike N --right C|P
  [--exchange SMART --currency USD --trading-class TC]` — one-shot snapshot of one option
  contract: price ticks + greeks (iv/delta/gamma/vega/theta/underlying_price) when the
  gateway delivers OptionComputation before SnapshotEnd.

## Success criteria (acceptance)

1. `omi option-chain AAPL` (paper `:4002` default) exits 0 printing
   `{underlying, conid, chains: [{exchange, trading_class, multiplier, expirations, strikes}]}`
   — default server-side exchange filter `SMART`; `expirations` and `strikes` sorted
   ascending (gateway returns unordered sets; agents need determinism).
2. `--exchange ""` ⇒ all exchanges (multiple rows); `--exchange CBOE` ⇒ that exchange only.
   The filter is the reqSecDefOptParams exchange parameter (server-side passthrough), not a
   client-side post-filter.
3. Unknown symbol ⇒ existing `not_found` envelope (empty contract_details), non-zero exit.
4. `omi option-quote --symbol AAPL --expiry 20260918 --strike 250 --right C` exits 0 within
   the bounded snapshot drain, printing
   `{contract: {symbol, expiry, strike, right, exchange, currency, multiplier,
   trading_class?}, delayed, ticks: {…price ticks…}, greeks: {…}}`.
   **Greeks are best-effort**: every greeks key omitted when the gateway never sends
   OptionComputation before SnapshotEnd — absence is NOT an error, output stays valid with
   `greeks` empty/omitted (arch pins which).
5. Offline-frozen arg validation, pre-connect: `--right` ∈ {C, P} (case-insensitive),
   `--strike` > 0, `--expiry` = 8-digit YYYYMMDD; violations ⇒ usage/config envelope,
   non-zero exit, no connection attempt.
6. Read-only invariant: no order/write calls anywhere in the new modules; `trade.rs`
   untouched; existing frozen suite green and byte-identical.
7. `cargo build` · `cargo clippy --all-targets -- -D warnings` · `cargo test` green.
8. **Merge gate (operator acceptance, paper `:4002`)**: `omi option-chain AAPL` shows
   plausible expirations/strikes; `omi option-quote` on a liquid near-month AAPL contract
   shows price ticks (greeks presence recorded as an observation either way). Tiger `:4001`
   support for reqSecDefOptParams is a journaled live observation, NOT an acceptance
   blocker (same class as the reqPnLSingle open observation).

## Scope

- `src/cli.rs`: `OptionChain(OptionChainArgs)`, `OptionQuote(OptionQuoteArgs)` variants.
- `src/ib/option_chain.rs` (new): conid resolve (contract.rs pattern) + End-bounded
  `option_chain` drain + pure chain-shaping seam (incl. sorting).
- `src/ib/option_quote.rs` (new): OptionBuilder contract + snapshot drain (quote.rs
  pattern) + pure greeks-extraction seam (OptionComputation → JSON, omit-if-absent).
- `src/ib/mod.rs` + `src/main.rs` wiring.
- Docs truth rides the PR: AGENTS.md + CLAUDE.md Phase-2 line amended from "no options" to
  "no option ORDERS" (data read now exists; writes still STK-only).
- No new dependency (ibapi 3.1 already carries the full surface).

## Non-scope (explicitly NOT this feature)

- No option orders (step 3), no BAG/spread/combos (step 4).
- No chain-wide batch quoting (N market-data-line fan-out = pacing risk; the agent composes
  `option-chain` → `option-quote`).
- No option historical data, no streaming subscriptions.
- No non-STK underlyings (STK guard like quote's), no FOP (futures options).
- No client-side expiry/strike filter flags on `option-chain` v1 — the agent filters JSON.
- No variadic option-quote v1 (ADR-0013-style N-shaping is a v2 candidate).

## Resolved decisions (locked)

- D1 **New commands, NOT `quote --sec-type OPT`** (operator-approved /think): quote.rs
  N-shaping output is frozen byte-identity (ADR 0013); extending it must touch frozen
  specs; new commands have zero freeze conflict. The tradeoff is not close.
- D2 **Chain default `--exchange SMART`** — bounds output to the useful row(s); `""` opts
  into the full multi-exchange dump (index underlyings are large).
- D3 **Greeks best-effort** — all-Option fields, omit-if-absent, never an error: under
  delayed md + snapshot mode some farms never push OptionComputation. This is the
  feature's most fragile assumption, deformed into the design instead of assumed away.
- D4 **option-quote is N=1** — no variadic v1.
- D5 **`--trading-class` optional passthrough** — disambiguates e.g. SPX vs SPXW; default
  empty (gateway resolves).
- D6 **md-type reuses the global `--md-type`** (default delayed), same
  switch_market_data_type call as quote.rs.
- D7 **Deterministic sorting** — expirations + strikes ascending in chain output.
- D8 **Two independently-mergeable cards** — card 01 option-chain, card 02 option-quote;
  neither depends on the other's code.

## Risks / fragile assumptions

- **OptionComputation may never arrive** under delayed+snapshot ⇒ D3 tolerates; acceptance
  passes on price ticks alone.
- **Tiger gateway (`:4001`) reqSecDefOptParams support unknown** — paper-first acceptance;
  live is a journaled observation (reqPnLSingle precedent).
- **Chain size**: full-exchange dumps for index underlyings run to hundreds of strikes —
  SMART default bounds v1; no pagination.
- **Bounded-ness verified in crate source**: chain drain ends via
  SecurityDefinitionOptionParameterEnd → Error::EndOfStream
  (stream_decoders.rs:50) — same End-bounded class as the completed-orders drain
  (ADR 0015/0016); quote drain is SnapshotEnd-bounded (quote.rs class). Whether the chain
  drain also wants an ADR-0012-style timeout wrap is arch's call (the wedge dossier says:
  every wait bounded).
- Rollback: purely additive (2 modules + 2 CLI variants + docs line); one revert removes it.

## Verification

- Offline frozen spec: pure seams (chain shaping incl. sorting; greeks extraction incl.
  omit-if-absent), arg-validation matrix (right/strike/expiry), `--help` lists both
  commands, dead-port connection envelope, existing suite untouched.
- Live (operator, paper `:4002`): criterion 8. Tiger `:4001` observation journaled
  opportunistically.
