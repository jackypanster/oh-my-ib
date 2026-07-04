# arch — option-orders (Phase 2 step 3: single-leg option writes)

Binding decisions in **ADR 0020**; glossary delta in `CONTEXT.md`. Safety architecture is
ADR 0017/0018 VERBATIM — this feature adds surface, not machinery. Write review polarity:
all new write code in `trade.rs`; polarity grep must stay clean.

## Design shape (five touched files + two doc amendments, no new dependency)

| file | change |
|---|---|
| `src/cli.rs` | `OptionBuy(OptionOrderArgs)`, `OptionSell(OptionOrderArgs)` (kebab: `option-buy`/`option-sell`); struct verbatim in §CLI |
| `src/ib/trade.rs` | pure seams `build_option_order` + `shape_option_order_ack`; gateway fns `option_buy`/`option_sell`; **place-core extraction** (§Core) |
| `src/ib/option_quote.rs` | **visibility-only**: `fn normalize_right` / `fn parse_expiry` → `pub(crate)` — no other change |
| `src/ib/mod.rs` | re-exports: `option_buy, option_sell, build_option_order, shape_option_order_ack` |
| `src/main.rs` | two dispatch arms |
| `AGENTS.md` + `CLAUDE.md` | §Docs amendment below, VERBATIM, nothing else |

NOT touched: every read module's behavior, `output.rs`, `error.rs`, `config.rs`, all
existing tests (stk 6-key ack + gate matrix stay frozen-green — they are D7's regression net).

## ibapi facts (source-verified, 2026-07-04)

- `ExpirationDate::new(y, m, d)`; `Display` = zero-padded `{:04}{:02}{:02}` ⇒
  `.to_string() == "20260918"` (types.rs:291/378) — the builder writes it into
  `Contract.last_trade_date_or_contract_month`.
- `OptionRight::{Call, Put}` (types.rs:188).
- Builder chain (options-read-proven): `Contract::call(sym)|::put(sym)` `.strike(px)`
  `.expires_on(y, m, d)` `.on_exchange(ex)` `.in_currency(cur)` [`.trading_class(tc)`]
  `.build()` ⇒ `security_type: SecurityType::Option`, `right: Some(..)`,
  `multiplier: "100"`, SMART/USD defaults.
- Placement machinery (stk-proven live): `client.next_order_id()` (ADR 0018 local
  allocator), `place_order(id, &contract, &order)` → `PlaceOrder::OrderStatus|OpenOrder`
  first-ack under `timeout_iter_data(TAKE_FIRST_TIMEOUT)`; `Order { action,
  total_quantity, tif: Day, order_type: "LMT", limit_price: Some(px) }`.

## CLI (impl copies verbatim)

```rust
/// omi option-buy --symbol AAPL --expiry 20260918 --strike 250 --right C --qty 1 --limit 5.50
#[derive(Args, Debug)]
pub struct OptionOrderArgs {
    /// Underlying ticker symbol, e.g. AAPL
    #[arg(long)]
    pub symbol: String,
    /// Expiry date, 8-digit YYYYMMDD
    #[arg(long)]
    pub expiry: String,
    /// Strike price (finite, > 0)
    #[arg(long)]
    pub strike: f64,
    /// Right: C|CALL or P|PUT (case-insensitive)
    #[arg(long)]
    pub right: String,
    /// Quantity in whole contracts (>= 1)
    #[arg(long)]
    pub qty: f64,
    /// Limit price (REQUIRED — v1 is LMT-only, no MKT; finite, > 0)
    #[arg(long)]
    pub limit: f64,
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

Enum doc-comments: `/// Place a single-leg option BUY (LMT/DAY; paper default; live needs
--live + OMI_ALLOW_LIVE=1)` — sell mirrors.

## Component design (impl follows verbatim; ALL in `src/ib/trade.rs`)

```rust
/// Pure, FROZEN seam: validated params → the exact (Contract, Order). LMT-only (v1, D2):
/// order_type always "LMT", limit_price always Some, TIF always Day. Contract via the
/// options-read builder chain (SMART/USD/x100 defaults; trading_class when given).
pub fn build_option_order(symbol: &str, expiry: (u16, u8, u8), strike: f64,
    right: OptionRight, trading_class: Option<&str>, exchange: &str, currency: &str,
    side: Action, quantity: f64, limit: f64) -> (Contract, Order)

/// Pure, FROZEN seam: the 9-key option ack. Echoes the request (expiry as the original
/// YYYYMMDD string; right normalized "C"|"P") + order_id/status from allocation + first ack.
pub fn shape_option_order_ack(order_id: i32, status: &str, symbol: &str, expiry: &str,
    strike: f64, right: &str, action: &str, quantity: f64, limit_price: f64) -> Value
```

**§Core — place-core extraction (D7).** Extract from today's `place()` the contract-agnostic
steps 2-6 (gate → connect → allocate → place_order → bounded first-ack loop) into:

```rust
/// Shared placement core (stk + option). Behavior byte-identical to the pre-refactor
/// stk path — the frozen stk suite asserts it. `ack` maps (order_id, status) → the
/// verb-specific ack JSON, so the ack SHAPES stay in their pure seams.
fn place_core(cfg: &Config, ctx: &str, contract: &Contract, order: &Order,
    ack: impl Fn(i32, &str) -> Value) -> Result<Value, AppError>
```

`place()` (stk) keeps its validation then calls the core with a closure over
`shape_order_ack`; `place_option()` validates (below) then closes over
`shape_option_order_ack`.

**Gateway fns** `option_buy(cfg, &OptionOrderArgs)` / `option_sell(..)` → `place_option(cfg,
args, Action::Buy|Sell, "option-buy"|"option-sell")`:

1. **Local validation FIRST (usage, offline; ordering frozen: usage < config < connection)**:
   - `right`: `super::option_quote::normalize_right(&args.right)` (pub(crate), D6) else usage;
   - `expiry`: `parse_expiry(&args.expiry)` (pub(crate)) else usage;
   - `strike`: `is_finite() && > 0.0` else usage;
   - `qty`: `is_finite() && >= 1.0 && fract() == 0.0` (whole contracts, D4) else usage;
   - `limit`: `is_finite() && > 0.0` else usage.
2. `require_live_write_gate(cfg)` (VERBATIM reuse).
3. `super::connect(cfg)` → `client.next_order_id()` (ADR 0018) → `build_option_order` →
   `place_core` with the 9-key ack closure.
4. No retry. No MKT arm. Timeout envelope wording = stk parity (names order id + `omi
   orders` + forbids blind retry), context `"option-buy"`/`"option-sell"`.

## Docs amendment (impl copies VERBATIM)

In `AGENTS.md` **and** `CLAUDE.md`, replace the sentence
`All other commands remain read-only; no modify, no option ORDERS, no combos yet (options DATA is readable: \`option-chain\`/\`option-quote\`).`
with:

> All other commands remain read-only; no modify, no combos yet. Options: DATA readable
> (`option-chain`/`option-quote`); single-leg option ORDERS exist
> (`option-buy`/`option-sell`, LMT/DAY only) behind the same gates.

Nothing else in either file changes (docs tests assert coarse markers only — still safe).

## Freeze coverage (pinned for pipeline-task — ONE card)

- **Frozen (`tests/option_orders_command.rs`, offline)**: `build_option_order` exact
  Contract/Order fields (call/buy + put/sell × trading_class present/absent; expiry
  `"20260918"` string; multiplier `"100"`; LMT/Some(limit)/Day always);
  `shape_option_order_ack` exact 9 keys; gate matrix (both verbs: live-no-env ⇒ config,
  `--port 4001` no-env ⇒ config, env+live+dead ⇒ connection, paper+dead ⇒ connection);
  validation matrix (qty 0 / -1 / 1.5 / inf; limit missing (clap usage) / 0 / inf; right X;
  expiry dashed; strike inf — all usage, all pre-connect); `--help` lists both verbs;
  verb `--help`s succeed.
- **Review must READ**: the place-core extraction (stk byte-identity — diff `place()`
  carefully), containment grep (write symbols only in trade.rs), the option_quote.rs
  visibility-only diff (exactly two `pub(crate)` tokens added), docs amendment verbatim,
  no-retry/no-MKT absence.
- **Live (operator, paper `:4002`, criterion 10)**: far-LMT option-buy → orders → cancel →
  completed-orders Cancelled → positions unchanged. Paper options-permission rejection =
  journaled observation, operator decides.

## Risks re-checked

- place-core refactor: ONLY behavior-preserving motion; frozen stk tests (gate matrix, ack
  shapes, validation) run offline and must stay green — any drift is a red suite, caught
  before PR.
- OptionRight/Action are ibapi types in a frozen pure-seam signature — same posture as the
  stk spec (quote_ticks.rs precedent); pre-freeze verbatim-compile check MANDATORY
  (options-read seq=5 lesson).
- Paper rejects option order (permission/entitlement): clean envelope path already exists
  (`place_order` err → data; order-status rejection arrives as the first ack — visible, not
  silent). Journal + stop for operator at the merge gate anyway.
