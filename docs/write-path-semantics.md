# write-path-semantics — reference-behavior audit

> Source of truth for every field `omi` actually sends to the Tiger/IB gateway on a write
> (`buy`/`sell`/`option-buy`/`option-sell`/`option-combo`/`option-close`/`cancel`). Frozen by
> `tests/write_path_semantics_doc.rs` (ADR 0025). DOC-ONLY audit (D6): a value that looks WRONG is
> registered `⚠️`, never fixed here — a fix is its own feature.
>
> **Field sources**: `Order` built via `..Default::default()` → ibapi's CUSTOM `impl Default`
> (`ibapi/orders/mod.rs:478-624`); a derived `Default` would be false and silently stage orders.
> Builders: `src/ib/trade.rs` (`build_stk_order`/`build_option_order`/`build_combo_order`/
> `stamp_order_account`) + ibapi `Contract::stock/call/put/spread` (`ibapi/contracts/builders.rs`).
>
> **Verification tiers** — exactly one per row:
> - `✅ paper-probe` — observed on `:4002` paper (Tiger accepted the value; live order ack'd).
> - `📖 doc-cite` — pinned by an ibapi source line and/or the IB TWS API Order/Contract reference.
> - `⚠️ UNVERIFIED` — plausible but never observed against Tiger; carries a probe recipe below.

## Field inventory

7-column table. `our value` = the exact bytes/enum the gateway sees for a single-leg STK LMT BUY
unless the cell qualifies otherwise. `inert` = `Default::default()` value, sent but not
send-meaningful for our order shapes.

| field | our value | ibapi type/behavior | Tiger/IB reference semantics | why this value / deliberate divergence | boundary cases | verification tier |
|---|---|---|---|---|---|---|
| `action` | `Action::Buy`/`Sell` (verb-derived) | `Action` enum (`mod.rs:88`); default `Buy` (`:485`) | BUY/SELL side. SELL auto-shorts if qty > long (ibapi doc). | verb (`buy`/`option-sell`/…) is the only side authority; option-close DERIVES side from held position sign (ADR 0022, never user-declared). | SSHORT/SLONG are institution-only; never emitted. **combo scalar-vector**: `Order.action` is a SCALAR that IBKR multiplies into each `ComboLeg.action` to get the effective leg position — `--action buy` keeps leg actions as-written, `--action sell` INVERTS them (see `credit` row + risk register). | 📖 doc-cite |
| `total_quantity` | `f64`, user `--qty` (e.g. `1.0`) | `f64` (`mod.rs:90`); default `0.0` (`:486`) | shares (STK) / contracts (OPT) / combo units (BAG). | passed through verbatim; option-close derives `|position|` or bounded sub-qty (ADR 0022 §2). | fractional qty rejected upstream; over-close capped at `|position|`. | 📖 doc-cite |
| `order_type` | `"LMT"` (option/combo always; STK when `--limit` given) else `"MKT"` (STK only) | `String` (`mod.rs:92`); default `""` (`:487`) | LMT = limit_price is the cap; MKT = immediate market. | single-leg option is LMT-only (ADR 0020 D2); combo is LMT-only (ADR 0021). STK supports MKT. | no STOP/STP_LMT emission. | 📖 doc-cite |
| `tif` | `TimeInForce::Day` | `TimeInForce` (`mod.rs:101`); default `Day` (`:490`) | order alive for the trading day. | always Day (v1; ADR 0020 D2, ADR 0021). NOT explicit in `Order { .. }` literal — inherited from custom Default. | no GTC/IOC/FOK/OPG emission. | 📖 doc-cite |
| `limit_price` | `Some(px)` for LMT; `None` for STK MKT | `Option<f64>` (`mod.rs:95`); default `None` (`:488`) | LIMIT price cap. MKT ⇒ absent. | single-leg: always `Some` (option LMT-only); STK MKT ⇒ `None`. **combo**: `Some(px)` SIGN-FREE — see `credit` row. | non-finite/non-positive rejected upstream for options. | 📖 doc-cite |
| `account` | `AccountId` string, RESOLVED (e.g. `"U12345"`) | `String` (`mod.rs` account field); default `""` | routes the order to a specific account. | stamped at the SINGLE placement choke point (`place_with_client`, `trade.rs:317`) via `stamp_order_account` (`trade.rs:255`), OVERWRITING any prior value — the resolved account is the only authority (ADR 0024 §2). | never user-declared on the order; `--account` resolves first, then stamp. | ✅ paper-probe |
| `transmit` | `true` (inherited) | `bool` (`mod.rs:113`); default `true` (`:494`) | false ⇒ TWS stages the order WITHOUT sending it. | NOT in the `Order { .. }` literal — inherited from ibapi custom Default. A derived `Default` (`false`) would silently never send; the canary (d) pins it. | never set `false` anywhere in the write path. | 📖 doc-cite |
| `outside_rth` | `false` (inherited) | `bool` (default `false`, `mod.rs:500`) | true ⇒ eligible outside regular trading hours. | inherited; we never enable RTH. | STK MKT outside RTH behaves differently — not exercised. | 📖 doc-cite |
| `display_size` | `Some(0)` (inherited) | `Option<i32>` (default `Some(0)`, `mod.rs:498`, ibapi carries `// TODO - default to None?`) | IB: for Iceberg/Hidden orders, the visible block size. `0` is ibapi's chosen default semantics. | inherited unchanged. **Whether Tiger treats `Some(0)` as "show all" (no iceberg) or as a degenerate iceberg is UNVERIFIED** — see risk register. | we never construct an iceberg; if `0` triggers partial-display behavior it is a latent bug (D6 fix = set `None`). | ⚠️ UNVERIFIED |
| `what_if` | `false` (inherited) | `bool` (`mod.rs:322`); default `false` (`:562`) | true ⇒ the order is a margin/commission PREVIEW, not a real order. | inherited; never set `true`. canary (d) pins it. | `OrderBuilder::analyze()` flips this — NOT used by `omi`. | 📖 doc-cite |
| `origin` | `OrderOrigin::Customer` (inherited) | `OrderOrigin` (default `Customer`, `mod.rs:516`) | identifies the submitter; Customer = retail flow. | inherited; never changed. | institutional `Dealer`/`Provider` never emitted. | 📖 doc-cite |
| `exempt_code` | `-1` (inherited) | `i32` (default `-1`, `mod.rs:519`) | IB: exempt-from-SSR flag; `-1` = not-exempt (the documented non-actionable value). | inherited; never changed. | `>= 0` would assert SSR exemption — never emitted. | 📖 doc-cite |
| `symbol` | underlying ticker, e.g. `"AAPL"` | `Contract.symbol`; back-filled for BAG | contract identity. STK/OPT: from `--symbol`. **combo**: `SpreadBuilder` leaves it `""`; `build_combo_order` back-fills `contract.symbol = underlying` (`trade.rs:572`). | combo back-fill is load-bearing — without it the BAG has no underlying. | case-normalized uppercase upstream. | 📖 doc-cite |
| `security_type` | `Stock` / `Option` / `Spread` (BAG) | `SecurityType`; set by each builder's `build()` | IB contract class. STK ⇒ `Stock` (`builders.rs:60`); OPT ⇒ `Option` (`:207`); BAG ⇒ `Spread` (`:592`). | chosen by the verb/contract ctor, never user-tunable. | no FUT/CASH/BOND emission. | 📖 doc-cite |
| `exchange` | `"SMART"` (default) or `--exchange` override | `Exchange` (`builders.rs`); StockBuilder/OptionBuilder/SpreadBuilder all default `"SMART"` (`:23`,`:93`,`:513`) | IB smart-routing. option/combo honor `--exchange`; STK uses the builder default. | parity with read path (`quote`/`contract`). | none — SMART is the universal default. | 📖 doc-cite |
| `currency` | `"USD"` (default) or `--currency` override | `Currency`; defaults `"USD"` (`:24`,`:94`,`:514`) | quote currency. option/combo honor `--currency`; STK uses the builder default. | Tiger gateway is USD-denominated. | non-USD never exercised against Tiger. | 📖 doc-cite |
| `multiplier` | `"100"` (options) | `String` on Contract; OptionBuilder default `100` (`builders.rs:95`/`:110`), serialized via `.to_string()` at `build()` (`:213`) | IB option contract multiplier (shares per contract). | fixed `100`; STK/BAG leave it as Contract default (empty). | never overridden; US equity options are 100. | 📖 doc-cite |
| `strike` | `f64`, e.g. `240.0` (options only) | `f64` on Contract (`OptionBuilder.strike`, `builders.rs:120-124`); serialized at `build()` (`:208`) | option strike price. | passed through from `--strike`; positive-finite validated upstream. | STK/BAG: Contract default (0.0), inert. | 📖 doc-cite |
| `right` | `Some(OptionRight::Call)`/`Put` (options only) | `Option<OptionRight>` on Contract; set at `build()` (`:209`) | C = call, P = put. | derived from verb (`option-call`/`option-put`/leg DSL); option-close takes it from the matched position. | STK/BAG: `None`, inert. | 📖 doc-cite |
| `credit` (combo net-limit sign) | combo `limit_price` is SIGN-FREE: `build_combo_order` passes `Some(limit)` unchanged (`trade.rs:577`); `--limit` accepts negative/zero/positive finite (CLI `allow_hyphen_values`, `cli.rs:237`) | `Option<f64>` (`mod.rs:95`); default `None` (`:488`) | **IBKR reference = ACTION-RELATIVE** (TWS "Notes on Combination Orders"): the net-limit sign is read TOGETHER with `Order.action`, and `Order.action` ALSO multiplies each `ComboLeg.action` (scalar-vector). Concretely, for a call credit spread with strikes L<H (bear call = SELL L / BUY H): **BUY-credit** = `--action buy` + `SELL L / BUY H` legs + **negative** limit (effective legs unchanged); **SELL-credit** = `--action sell` + `BUY L / SELL H` legs + **positive** limit (effective legs inverted to `SELL L / BUY H`). There is NO global "negative = credit" rule. | omi's CLI help string `--limit ... negative = credit` (`cli.rs:236`) is a SIMPLIFICATION that only matches the BUY-action case; it does NOT mention the scalar-vector leg inversion nor the positive-limit SELL-credit case. omi passes `--limit` and the leg DSL through verbatim, so the OPERATOR must supply a coherent action/leg-vector/sign triple per IBKR. | single-leg paths never emit a negative limit (validated upstream). Whether Tiger enforces IBKR's action-relative sign convention AND the scalar-vector leg inversion — and whether omi's help-string simplification misleads operators — is UNVERIFIED. See risk register. | ⚠️ UNVERIFIED |
| inert tail (~70 remaining `Order` fields) | ibapi `Default::default()` each: empty string / `0` / `0.0` / `false` / `None` / `vec![]` | full list `mod.rs:73-476`; defaults `:478-624` | not send-meaningful for our shapes (no OCA, no algo, no trailing, no delta-neutral, no volatility, …). | inherited en masse via `..Default::default()`; never named in the write path. | a future verb that sets one of these MUST add a row here (anti-rot guard (c) fires on builder output diff). | 📖 doc-cite |

## Placement choke point

`stamp_order_account` mutates `order.account` IN PLACE at `place_with_client` (`trade.rs:317`) — the
single gate every placement verb funnels through, so no current or future verb can skip the account
stamp. This is a post-build gateway-path mutation, NOT builder output, so it is invisible to the
anti-rot serde-diff (c); covered by the required-field list (b) + review.

## ⚠️ Risk register

One entry per `⚠️` row. Recipes are runnable on `:4002` (paper, ungated) during a live US session —
DEFERRED (D2): the doc ships with recipes; executing them is an operator lifecycle, not a merge gate.

### display_size = Some(0)

- **Concern**: ibapi's own source carries `// TODO - default to None?` at `mod.rs:498`. If Tiger
  interprets `Some(0)` as "iceberg with visible block 0" rather than "no iceberg / show all", every
  `omi` order would be sent with degenerate partial-display semantics.
- **Probe recipe** (`:4002`, paper):
  1. Place a far-from-market single-leg LMT so it rests open:
     `omi option-buy --symbol AAPL --expiry 20260918 --strike 240 --right C --qty 1 --limit 0.05`
  2. `omi orders` — observe the open order.
  3. **Confirms `Some(0)` is benign** if: the order is accepted at the full `--qty` with no partial/
     iceberg display flag, and behaves identically to a hand-placed TWS limit order at the same price.
  4. `omi cancel <id>` to clean up.
- **Fallback (if `0` triggers iceberg semantics)**: a separate feature sets `display_size = None` in
  the builders (D6 — do NOT fix here). Record the observed Tiger behavior in that feature's ADR.

### combo net-limit sign (credit) — action-relative + scalar-vector

- **Concern**: two independent things are unverified against Tiger. (a) **Sign convention**: does
  Tiger enforce IBKR's action-relative net-limit sign (BUY-credit ⇒ negative; SELL-credit ⇒ positive)?
  (b) **Scalar-vector**: does Tiger compute the effective leg position as `Order.action × ComboLeg.action`
  (so `--action sell` INVERTS the leg vector) per IBKR's TWS combo lesson? omi's CLI help string
  `--limit ... negative = credit` (`cli.rs:236`) is a simplification that ignores both subtleties —
  it only matches the BUY-credit case and silently inverts meaning for SELL-action combos.
- **Probe recipe** (`:4002`, paper, US session so the option chain resolves). Three probes on a call
  credit spread with strikes L=240 < H=250 (bear call = `SELL 240 / BUY 250`). Each probe NAMES the
  effective position it actually creates under the IBKR scalar-vector model, so the operator can read
  off which convention Tiger follows:
  1. **BUY-credit per IBKR** — coherent baseline (negative limit, BUY-action, legs as-written):
     `omi option-combo --action buy --leg "SELL 1 AAPL 20260918 240 C" --leg "BUY 1 AAPL 20260918 250 C" --qty 1 --limit -0.05 --exchange SMART --currency USD`
     Effective legs (×+1) = `SELL 240 / BUY 250` = bear call = credit spread, **bought**. This is the
     same shape the frozen test `negative_net_limit_is_a_credit_and_builds` asserts at build time.
     `omi orders`. **Confirms Tiger accepts BUY-credit at a negative limit** if the BAG rests without a
     sign rejection.
  2. **SELL-credit per IBKR** — coherent opposite (positive limit, SELL-action, inverted leg vector):
     `omi option-combo --action sell --leg "BUY 1 AAPL 20260918 240 C" --leg "SELL 1 AAPL 20260918 250 C" --qty 1 --limit 0.05 --exchange SMART --currency USD`
     Effective legs (×−1) = `SELL 240 / BUY 250` = same bear call = credit spread, **sold**.
     `omi orders`. **Confirms Tiger follows IBKR's scalar-vector + positive-limit SELL-credit** if the
     BAG rests at the intended net credit (you receive $0.05) with no sign rejection.
  3. **Divergence probe** — the omi help-string literal applied to a SELL-credit leg vector
     (negative limit, SELL-action, inverted leg vector — same legs as probe 2):
     `omi option-combo --action sell --leg "BUY 1 AAPL 20260918 240 C" --leg "SELL 1 AAPL 20260918 250 C" --qty 1 --limit -0.05 --exchange SMART --currency USD`
     Effective legs (×−1) = `SELL 240 / BUY 250` = credit spread sold, BUT the negative limit is the
     IBKR SELL-debit sign (you pay). **Confirms the help-string divergence is real** if Tiger rejects,
     re-prices, or fills this as a debit where probe 2 (positive limit, same legs) was accepted.
  4. `omi cancel <id>` after each to clean up.
- **Reading the triplet**: probes 1+2 both green ⇒ Tiger follows IBKR (action-relative sign +
  scalar-vector). Probe 3 rejected where probe 2 accepted ⇒ Tiger enforces the sign convention and
  omi's `negative = credit` help string misleads for SELL-combos (separate feature to fix, D6). Probe 2
  rejected/repriced ⇒ Tiger does NOT follow the scalar-vector inversion, or uses a different sign rule;
  record what it does.
- **Fallback (any rejection/misprice)**: record the sign + scalar-vector convention Tiger actually
  enforces in a new ADR; fix is either the `build_combo_order` sign handling (`trade.rs:577`) or the
  CLI help string (`cli.rs:236`) — a separate feature (D6). Do NOT change the sign here.

## Anti-rot guard

The frozen spec's guard (c) diffs each builder's serialized `Order` against `Order::default()` and
fails if any differing field lacks a row above. Adding a new builder-set `Order` field without a row
here ⇒ red; the failure message names the field. The canary (d) pins the load-bearing defaults
(`transmit==true`, `outside_rth==false`, `what_if==false`, `tif==Day`, `display_size==Some(0)`,
`origin==Customer`, `exempt_code==-1`) so an ibapi bump that silently flips one turns the suite red
before it reaches a gateway.
