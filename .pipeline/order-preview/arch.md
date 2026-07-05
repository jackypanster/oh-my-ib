# arch — order-preview (whatIf order preview)

Status: arch (2026-07-05, feature order-preview). Consumes PRD.md. Binding decision → ADR 0026.
All PRD decisions are `✅ human-confirmed` or `📖 code-verified`; no arch-level human grill was
needed. The one external-reference claim (Tiger honors `what_if`) is tabled in CONTEXT.md §Reference
behavior (cannot be settled by code — live-acceptance only).

## Chosen shape

Branch to a non-transmitting whatIf placement **at the placement call-site, after the gate**, so the
real transmit path stays byte-identical and the write gate is reused unchanged. One new pure FROZEN
seam (`shape_preview`) + one new gateway fn (`preview_with_client`, review-by-reading). Flag plumbs
`GlobalOpts.preview → Config.preview`, read inside the six order verbs.

## Component boundaries (write locus = src/ib/trade.rs ONLY, per AGENTS.md)

| file | change | frozen? |
|---|---|---|
| `src/cli.rs` | add `pub preview: bool` to `GlobalOpts` (mirror `pub live: bool`, cli.rs:33) | flag parse — FROZEN (black-box) |
| `src/config.rs` | add `pub preview: bool` to `Config` (default false); in `merge_flags` add `self.preview = g.preview;` (mirror `g.live`, config.rs:116) | pure — FROZEN (unit) |
| `src/main.rs` | **none** — dispatch already passes `&config` to all six verbs (main.rs:77-85) | — |
| `src/ib/trade.rs` | (a) each verb branches `if cfg.preview`; (b) `preview_with_client` gateway fn; (c) `shape_preview` pure seam | (a)/(c) FROZEN; (b) review-by-reading |

## Data flow

```
omi --live buy AAPL 100 --limit 250 --preview
  └─ GlobalOpts{preview:true, live:true}            src/cli.rs   (parse)
  └─ Config{port:4001, preview:true}                src/config.rs merge_flags  (--live→port; g.preview→preview)
  └─ ib::buy(&cfg, args)                             src/main.rs  (UNCHANGED)
  └─ place(): validate → build_stk_order(contract,order)          src/ib/trade.rs
       └─ place_core: require_live_write_gate(cfg)  ← THE GATE (identical, untouched)
                     → connect → resolve_account
                     → if cfg.preview                              ← the ONLY new branch, AFTER the gate
                         ├ yes → preview_with_client(client, ctx, contract, order, account)
                         │        order.what_if = true → place_order(id, contract, order)
                         │        first OpenOrder(od) → shape_preview(contract, order, &od.order_state)
                         │        → { preview, what_if, action, contract, order, margin, commission, warning, status }
                         └ no  → place_with_client(...)            ← BYTE-IDENTICAL to today (frozen suites assert)
```

Combo/close reach `place_with_client` directly (they own a client for per-leg/position conid
resolution). Their branch is the SAME: swap `place_with_client` → `preview_with_client` at the call
site, AFTER their existing `require_live_write_gate`. Gate placement is never moved.

## The two new seams

- **`shape_preview(&Contract, &Order, &OrderState) -> Value` — pure, FROZEN.** Mirrors the existing
  "Pure, FROZEN seam: the ack JSON" (trade.rs:51). Reads the order echo from `Order`
  (`action`/`total_quantity`/`order_type`/`limit_price`), the contract echo from `Contract`
  (symbol/sec_type/expiry/strike/right/exchange/currency/conid; combo = BAG + legs), and margin/
  commission/warning/status from `OrderState`. `Option<f64>::None → JSON null`. The OrderState→key
  MAPPING lives here, so it is FROZEN. Frozen test constructs a real `OrderState { .. }` literal
  (derives `Default`, pub fields — ibapi mod.rs:1274; a value literal, NOT a mock) + the real built
  `(Contract, Order)` and asserts the exact envelope.
- **`preview_with_client(client, ctx, contract, order, account) -> Result<Value, AppError>` — gateway
  fn, review-by-reading (NOT frozen; needs a live gateway).** = `place_with_client` with two deltas:
  set `order.what_if = true` after the account stamp; on the first `OpenOrder(od)` return
  `shape_preview(contract, &order, &od.order_state)` instead of the per-verb `ack`. Same bounded
  first-ack loop, same timeout/UNKNOWN semantics. The per-verb `ack` closure is simply unused on the
  preview branch.

## Uniform envelope (stable keys, all six verbs)

```json
{ "preview": true, "what_if": true, "action": "BUY",
  "contract": { "symbol": "AAPL", "sec_type": "STK", "conid": 265598, "…": "…" },
  "order":    { "type": "LMT", "qty": 100, "limit": 250.0 },
  "margin":     { "init_change": <f64|null>, "maint_change": <f64|null>, "equity_with_loan_change": <f64|null> },
  "commission": { "value": <f64|null>, "min": <f64|null>, "max": <f64|null>, "currency": <str> },
  "warning": <str>, "status": <str> }
```

Sources (📖 code-verified ibapi `OrderState`): `initial_margin_change` / `maintenance_margin_change`
/ `equity_with_loan_change` / `commission` / `minimum_commission` / `maximum_commission` (all
`Option<f64>`), `commission_currency` / `warning_text` (`String`), `status` (`OrderStatusKind`).

## Freeze boundary (arch → task gate)

- **FROZEN** (task writes red tests over `spec-paths`): `--preview` flag parses on all six verbs
  (black-box `assert_cmd` → dead-port connection envelope, mirroring existing write-path parse tests);
  `Config.preview` merge (unit); `order.what_if == true` after the preview transform and `== false`
  on the real path; `shape_preview` exact envelope + OrderState→key mapping + `None→null`.
- **REVIEW-BY-READING / live-acceptance** (task must NOT freeze — no live gateway): that Tiger
  returns an `OpenOrder` whose `order_state` carries margin/commission under `what_if`, and that
  `what_if` does not transmit. Verified by the operator live-acceptance protocol (PRD criterion 8);
  the premise is tabled in CONTEXT.md §Reference behavior as a `⚠️ unverified` risk row.

## Non-scope (unchanged from PRD)

order modify · stop/bracket/GTC/TIF · any change to the real transmit path · server-side confirm
(lives in hermes) · relaxing preview to read-shaped/ungated (deferred, evidence-gated).
