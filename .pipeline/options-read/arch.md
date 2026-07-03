# arch — options-read (Phase 2 step 2: the options READ path)

Binding decisions in **ADR 0019**; glossary in `CONTEXT.md`. Every ibapi claim source-verified
(local crate `ibapi-3.1.0`, 2026-07-04). READ-ONLY feature: normal review polarity (the
standard read-only grep — write symbols stay confined to `trade.rs`, untouched here).

## Design shape (five touched files + two doc amendments, no new dependency)

| file | change |
|---|---|
| `src/cli.rs` | `OptionChain(OptionChainArgs)`, `OptionQuote(OptionQuoteArgs)` variants (clap kebab-cases to `option-chain`/`option-quote`); arg structs verbatim in §CLI below |
| `src/ib/option_chain.rs` | NEW — conid resolve + End-bounded chain drain (timeout-wrapped) + pure seam `shape_option_chain` + ibapi-free `ChainRow` |
| `src/ib/option_quote.rs` | NEW — option contract build + SnapshotEnd-bounded drain + pure seams `option_quote_greeks` / `shape_option_quote` + ibapi-free `GreeksRow` |
| `src/ib/mod.rs` | `mod option_chain; mod option_quote;` + re-exports (`option_chain, shape_option_chain, ChainRow, option_quote, option_quote_greeks, shape_option_quote, GreeksRow`) |
| `src/main.rs` | two dispatch arms |
| `AGENTS.md` + `CLAUDE.md` | red-line phrase amendment — VERBATIM text in §Docs amendment below, nothing else |

NOT touched: `quote.rs` (FROZEN byte-identity, ADR 0013 — reuse `quote_price_tick` via the
`super::` re-export, never edit the file), `trade.rs`, `output.rs`, `error.rs`, `config.rs`,
all existing tests. Verified: `tests/agents_md.rs`/`claude_md.rs` assert only coarse markers
(`agent-first`, `CONTRACT.md`, `OMI_ALLOW_LIVE`) — the docs amendment cannot break them.

## ibapi facts (source-verified, 2026-07-04)

- `client.option_chain(symbol: &str, exchange: &str, security_type: SecurityType, contract_id: i32)
  -> Result<Subscription<OptionChain>, Error>` (contracts/sync.rs:267) = reqSecDefOptParams.
- `OptionChain` rows: `{underlying_contract_id: i32, trading_class: String, multiplier: String,
  exchange: String, expirations: Vec<String>, strikes: Vec<f64>}` (contracts/mod.rs:894) —
  one row per (exchange, trading_class); **unordered sets** (sort ours, PRD D7).
- Termination: `SecurityDefinitionOptionParameterEnd` → `Error::EndOfStream`
  (contracts/common/stream_decoders.rs:50), which the subscription iterator converts to a
  **clean `None`** (subscriptions/sync.rs:171-174, 185-188) — the completed-orders drain class.
  `timeout_iter_data(TAKE_FIRST_TIMEOUT)` + Instant-classified `None` arms apply (ADR 0016).
- Option contract: `Contract::call(sym).strike(px).expires_on(y, m, d).build()`
  (contracts/mod.rs:276/292 → builders.rs) — defaults exchange SMART, currency USD,
  **multiplier 100**; optional `.on_exchange(..)`, `.in_currency(..)`, `.trading_class(..)`.
  `Contract::put` mirrors. (The flat `Contract::option(..)` ctor skips multiplier — builder wins.)
- Greeks arrive on the EXISTING snapshot drain as `TickTypes::OptionComputation(OptionComputation)`
  (market_data/realtime/mod.rs:340). `OptionComputation.field: TickType` distinguishes
  computation rows: `BidOption=10, AskOption=11, LastOption=12, ModelOption=13,
  DelayedBidOption=80..DelayedModelOption=83, CustOptionComputation=53`
  (contracts/tick_types/mod.rs). Value fields all `Option<f64>`:
  `implied_volatility, delta, gamma, vega, theta, option_price, underlying_price`
  (+ `present_value_dividend`, `tick_attribute` — excluded from output, ADR 0019).
- Price ticks for the option arrive as `TickTypes::Price(..)` exactly like stock quotes
  (delayed variants included) — `quote_price_tick` (pub, re-exported) applies unchanged.

## CLI (impl copies verbatim)

```rust
/// omi option-chain AAPL [--exchange SMART]
#[derive(Args, Debug)]
pub struct OptionChainArgs {
    /// Underlying ticker symbol, e.g. AAPL
    pub symbol: String,
    /// Exchange filter (server-side reqSecDefOptParams param); "" = all exchanges
    #[arg(long, default_value = "SMART")]
    pub exchange: String,
}

/// omi option-quote --symbol AAPL --expiry 20260918 --strike 250 --right C
#[derive(Args, Debug)]
pub struct OptionQuoteArgs {
    /// Underlying ticker symbol, e.g. AAPL
    #[arg(long)]
    pub symbol: String,
    /// Expiry date, 8-digit YYYYMMDD, e.g. 20260918
    #[arg(long)]
    pub expiry: String,
    /// Strike price (> 0)
    #[arg(long)]
    pub strike: f64,
    /// Right: C|CALL or P|PUT (case-insensitive)
    #[arg(long)]
    pub right: String,
    /// Exchange
    #[arg(long, default_value = "SMART")]
    pub exchange: String,
    /// Currency
    #[arg(long, default_value = "USD")]
    pub currency: String,
    /// Trading class, e.g. SPXW (optional; gateway resolves when absent)
    #[arg(long)]
    pub trading_class: Option<String>,
}
```

Command enum doc-comments: `/// Option chain (expirations × strikes) for an underlying` ·
`/// Snapshot quote + greeks for one option contract`.

## Component design (impl follows verbatim)

`src/ib/option_chain.rs`:

```rust
/// Plain ibapi-free chain row (SearchRow/PnlSingleRow/CompletedOrderRow pattern) — the
/// frozen test constructs these directly; offline-testable with no gateway.
pub struct ChainRow {
    pub exchange: String,
    pub trading_class: String,
    pub multiplier: String,
    pub expirations: Vec<String>,
    pub strikes: Vec<f64>,
}

/// Pure, FROZEN seam: rows → `{underlying, conid, chains: [...]}`. Sorts each row's
/// expirations (lexicographic == chronological for YYYYMMDD) and strikes (partial_cmp)
/// ascending, then rows by (exchange, trading_class) — full determinism (PRD D7).
/// Zero rows ⇒ `chains: []` (success — the gateway answered; agent sees empty).
pub fn shape_option_chain(underlying: &str, conid: i32, rows: Vec<ChainRow>) -> Value
```

Gateway fn `option_chain(cfg, args)` (review-by-reading):
1. connect → resolve underlying conid: `Contract::stock(&args.symbol).build()` →
   `contract_details` → **FIRST row's** `contract.contract_id` (ADR 0019 D4; SMART/USD stock
   builder defaults = quote/contract parity); empty ⇒ `not_found` (contract.rs mirror).
2. `client.option_chain(&args.symbol, &args.exchange, SecurityType::Stock, conid)` →
   **timeout-wrapped End-bounded drain** — the completed_orders.rs:83-128 skeleton verbatim:
   `timeout_iter_data(super::TAKE_FIRST_TIMEOUT)`, Instant-classified `None` arms
   (starved-window `None` ⇒ exit-6 `timeout` envelope naming reqSecDefOptParams; instant
   `None` ⇒ End received ⇒ success), `Some(Err(e))` ⇒ `data` envelope, context
   `"option-chain"`.
3. Map each `OptionChain` → `ChainRow` (field-for-field) → `Ok(shape_option_chain(..))`.

`src/ib/option_quote.rs`:

```rust
/// Plain ibapi-free greeks row. All fields optional — emitted key-by-key (omit-if-None).
pub struct GreeksRow {
    pub implied_volatility: Option<f64>,
    pub delta: Option<f64>,
    pub gamma: Option<f64>,
    pub vega: Option<f64>,
    pub theta: Option<f64>,
    pub option_price: Option<f64>,
    pub underlying_price: Option<f64>,
}

/// Pure, FROZEN seam (quote_price_tick's sibling): Some(GreeksRow) ONLY for
/// `TickTypes::OptionComputation` whose `field` is ModelOption or DelayedModelOption
/// (ADR 0019 D3 — the TWS-greeks rows); every other tick ⇒ None. Tests construct
/// `OptionComputation` directly (pub fields; quote_ticks.rs precedent).
pub fn option_quote_greeks(tick: &TickTypes) -> Option<GreeksRow>

/// Pure, FROZEN seam: assemble `{contract, delayed, ticks, greeks?}`. `contract` echo =
/// exact 8 keys {symbol, expiry, strike, right, exchange, currency, multiplier: "100",
/// trading_class (null when absent)}; `right` echoed normalized ("C"|"P"). `greeks` key
/// present IFF a model row arrived (last-write-wins); inside it only Some fields appear.
pub fn shape_option_quote(args: &OptionQuoteArgs, right: &str, delayed: bool,
    ticks: serde_json::Map<String, Value>, greeks: Option<GreeksRow>) -> Value
```

Gateway fn `option_quote(cfg, args)` (review-by-reading):
1. **Pre-connect validation** (offline-frozen, usage/config envelopes): `right` parses
   case-insensitively to Call (`"c"|"call"`) / Put (`"p"|"put"`) else usage; `strike > 0.0`
   else usage; `expiry` = exactly 8 ASCII digits parsing to y/m/d with m∈1..=12, d∈1..=31
   else usage.
2. connect → `switch_market_data_type` (cfg.md_type — quote.rs:32-39 verbatim, same
   `delayed` bool threading).
3. Build: `Contract::call(&args.symbol)` or `::put(..)` → `.strike(args.strike)` →
   `.expires_on(y, m, d)` → `.on_exchange(&args.exchange)` → `.in_currency(&args.currency)`
   [→ `.trading_class(tc)` when Some] → `.build()`.
4. `market_data(&contract).snapshot().subscribe()` → **bare `iter_data()` to SnapshotEnd**
   (quote.rs class — deliberately NOT timeout-wrapped, same rationale comment as
   quote.rs:44-45; ADR 0019 D2). Per tick: `quote_price_tick` hit ⇒ insert into `ticks`;
   `option_quote_greeks` hit ⇒ replace `greeks` (last-write-wins). Error context
   `"option-quote"`.
5. `Ok(shape_option_quote(..))`.

## Output shapes (frozen by the spec tests)

```json
// omi option-chain AAPL
{"underlying":"AAPL","conid":265598,
 "chains":[{"exchange":"SMART","trading_class":"AAPL","multiplier":"100",
            "expirations":["20260117","20260918"],"strikes":[100.0,105.0]}]}

// omi option-quote --symbol AAPL --expiry 20260918 --strike 250 --right c
{"contract":{"symbol":"AAPL","expiry":"20260918","strike":250.0,"right":"C",
             "exchange":"SMART","currency":"USD","multiplier":"100","trading_class":null},
 "delayed":true,
 "ticks":{"DelayedBid":248.1,"DelayedAsk":249.0},
 "greeks":{"implied_volatility":0.31,"delta":0.52,"gamma":0.01,"vega":0.4,"theta":-0.09,
           "option_price":12.4,"underlying_price":251.2}}
// greeks key ABSENT when no model computation arrived; inside greeks only Some fields.
```

## Docs amendment (impl copies VERBATIM)

1. `AGENTS.md` **and** `CLAUDE.md` — in the "Writes are gated" bullet, replace the sentence
   `All other commands remain read-only; no modify, no options, no combos yet.` with:

   > All other commands remain read-only; no modify, no option ORDERS, no combos yet
   > (options DATA is readable: `option-chain`/`option-quote`).

2. `AGENTS.md` §What this is — stk-orders leftover (its criterion 9 missed this line):
   replace `Read-only (no order-placement code).` with:

   > Reads everything; writes exist but are Phase-2 gated (see Hard safety rules).

Nothing else in either file changes. (Docs tests assert only coarse markers — verified safe.)

## Freeze coverage (pinned for pipeline-task)

- **Frozen (offline; suggest `tests/option_chain_command.rs` + `tests/option_quote_command.rs`,
  card-scoped test-name filters):**
  - `shape_option_chain`: unsorted input rows ⇒ expirations/strikes ascending + rows ordered
    by (exchange, trading_class); exact envelope keys; zero rows ⇒ `"chains":[]`.
  - `option_quote_greeks`: ModelOption row ⇒ Some (values mapped); DelayedModelOption ⇒ Some;
    Bid/Ask/Last/Cust computation rows ⇒ None; non-computation tick ⇒ None; None-valued
    fields omitted downstream.
  - `shape_option_quote`: greeks key present iff Some; 8-key contract echo (trading_class
    null when absent); right normalized "C"/"P"; ticks pass-through.
  - **Validation matrix (pre-connect, dead port NOT reached):** `--right X` ⇒ usage;
    `--strike 0`/negative ⇒ usage; `--expiry 2026091` / `2026-09-18` / `20261332` ⇒ usage.
  - Dead-port with valid args ⇒ `code="connection"` for both commands.
  - `--help` lists `option-chain` + `option-quote`.
- **Review-by-reading:** gateway fns (chain drain timeout-wrapped, quote drain bare;
  conid-first-row rule; no writes anywhere — normal polarity grep); docs amendment
  verbatim-match.
- **Live (operator, paper `:4002`, PRD criterion 8):** `omi option-chain AAPL` plausible;
  `omi option-quote` liquid near-month AAPL ⇒ price ticks (greeks = recorded observation).
  Tiger `:4001` reqSecDefOptParams support = journaled observation, never a blocker.

## Risks re-checked

- SMART may be an empty reqSecDefOptParams filter on some gateway builds ⇒ operator escape
  hatch is `--exchange ""` (all exchanges); acceptance notes which behavior the paper gateway
  exhibits.
- Model greeks may never arrive under delayed+snapshot ⇒ greeks key absent, output remains
  valid (PRD D3) — NOT an error, NOT a frozen assertion.
- reqSecDefOptParams End may never come on wedgy builds (reqCompletedOrders precedent) ⇒
  timeout wrap guarantees a clean exit-6, never a hang.
- Chain output size: index underlyings across all exchanges are large — SMART default bounds
  v1; documented, no pagination.
- Rollback: purely additive (2 modules, 2 CLI variants, 2 doc sentences) — one revert.
