# Architecture — write-path-semantics

Doc-only feature. Product = `docs/write-path-semantics.md` (a reference-behavior audit table), guarded
by ONE new frozen test. No `src/` change. One card.

## Code-first verification results

1. **ibapi `Order` = ~90 fields with a CUSTOM `impl Default`** (`mod.rs:478`), NOT derived → `transmit:
   true` (`:494`), `outside_rth: false` (`:500`), `display_size: Some(0)` (`:498`, crate `// TODO`),
   `what_if: false` (`:562`), `origin: Customer` (`:516`), `exempt_code: -1` (`:519`). A *derived*
   Default would zero `transmit` ⇒ orders staged-not-sent. [verified]
2. **Explicit `Order` fields set by `trade.rs`**: `action`, `total_quantity`, `order_type`, `tif`,
   `limit_price` (the three `Order { … }` builder literals) + `account` (`stamp_order_account`,
   `trade.rs:255`). [verified]
3. **Contract defaults**: `Contract::stock` → SMART/USD (`builders.rs:23-24`); `call`/`put` → SMART/USD/
   `multiplier=100` (`:93-95,108-110`); `spread` (BAG) → USD, multiplier `None` (`:239-240`), `symbol`
   back-filled (`trade.rs:571`). STK/OPT/BAG security types. [verified]
4. **Frozen tests assert pure construction only** (`option_orders_command.rs:65` `order_type=="LMT"`;
   `option_combo_command.rs:119`); gateway path unfrozen (`trade.rs:259`); no suite asserts
   `order.account` (order-account-stamp arch). [verified]
5. **`Order.account` already `✅`** — order-account-stamp paper probe (journal seq=4/6: Tiger accepts an
   explicit account). **combo `limit_price` sign-free "negative = credit"** (`trade.rs:577`, ADR 0021)
   has **no recorded verification** → `⚠️` seed. [verified]

## Design (pins PRD D1-D6; full detail in ADR 0025)

### Deliverable (impl-paths) — `docs/write-path-semantics.md`
7-column table (schema: ADR 0025 §1) + `## ⚠️ Risk register`. Row classes: **explicit-set** /
**load-bearing-default** / one **inert-tail catch-all**. `account` row pre-filled `✅`; `transmit` and
combo-credit rows seeded `⚠️` with probe recipes. Agent-first (AGENTS.md §Authoring).

### Frozen spec (spec-paths) — `tests/write_path_semantics_doc.rs`
Coverage + default-canary (ADR 0025 §3). **GOTCHA: runtime `std::fs::read_to_string(concat!(env!(
"CARGO_MANIFEST_DIR"), "/docs/write-path-semantics.md"))`, NOT `include_str!`** (absent-file compile
trap — the spec must COMPILE and FAIL). RED now (doc absent) → GREEN when the doc is complete.

### Boundaries
- `spec-paths = tests/write_path_semantics_doc.rs`; `impl-paths = docs/write-path-semantics.md`.
  Disjoint ✓ (CONTRACT invariant).
- **NO `src/ib/trade.rs` change** (D6). **NO read path.** **NO process/skill change** (D1 →
  SKILL-PROPOSAL).

## Freeze plan handed to task (advisory)

- **One card.** spec-paths: `tests/write_path_semantics_doc.rs` (RED). impl-paths:
  `docs/write-path-semantics.md`.
- **Card verify**: `cargo test --test write_path_semantics_doc`. `full-verify` unchanged
  (`[cargo build, cargo test]`).
- **Required-field token list** for assertion (b): `action, total_quantity, order_type, tif,
  limit_price, account, transmit, outside_rth, display_size, what_if, origin, exempt_code, exchange,
  currency, multiplier, symbol, strike, right, security_type, combo net-limit`.
- **Canary asserts** (d): `Order::default()` → `transmit==true, outside_rth==false, what_if==false,
  tif==TimeInForce::Day, display_size==Some(0), origin==OrderOrigin::Customer, exempt_code==-1`.
- **Anti-rot source-scan** (c): read `src/ib/trade.rs`; extract `order\.(\w+)\s*=` + fields inside
  `Order { … }` literals; assert each is a documented row. Accept mild regex fragility — trade.rs is the
  single small write file (AGENTS.md hard rule).
- **Freeze coverage note for review**: structural coverage + default-pin are frozen; each row's SEMANTIC
  truth (reference-semantics / boundary columns) is **review-by-reading + the deferred `⚠️` probes**.

## ADR

- **0025** — write-path-semantics: a test-guarded living reference doc.
