# PRD — order-preview (whatIf preview for the write path)

Status: prd (2026-07-05, feature order-preview)
Origin: `/think` (2026-07-05) → the natural-language → hermes → live-money loop has no
"intent → preview → human-confirm → execute" step. Approved single highest-ROI todo.
Branch (eventual PR): `feat/order-preview`. Trunk: `main`.

## Problem

- `src/ib/trade.rs` fires orders immediately: `place(...)` → `place_core` → `place_with_client`
  (`trade.rs:317`) calls `client.place_order(...)` and returns the first `OrderStatus`/`OpenOrder`
  ack. There is **NO whatIf / preview / dry-run anywhere** (grep-confirmed across `src/`).
- The consumer is an **LLM (hermes) parsing free-text from Telegram**. The operator runs the **Tiger
  gateway LIVE on `:4001`** (AGENTS.md) — the default paper `:4002` likely does not exist on that
  host, so in practice every hermes order is **real money**. An LLM misparse fires blind.
- hermes cannot show a trustworthy confirm card today: it can only echo *its own* parse (the thing
  that may be wrong). It cannot show the **gateway-resolved contract**, nor **margin / commission**.
- IB's `Order.what_if=true` (`ibapi-3.1.0/src/orders/mod.rs:322`) is a **non-transmitting** margin/
  commission query — exactly the missing primitive. `OrderState` already returns every margin/
  commission field as `Option<f64>` (`mod.rs` OrderState) so "present-if-populated, else null" is the
  crate's native shape.

## Goal

Add a **`--preview`** path so `omi <order-verb> … --preview` runs the whatIf query (transmits
nothing) and returns **one uniform, stable JSON preview envelope**: the gateway-resolved contract
(ground truth), the order-param echo (always), and margin/commission/warning **when the gateway
populates them, `null` otherwise**. This gives hermes a ground-truth confirm card before any real
fire — and the design **ships value even if Tiger ignores `what_if`** (echo + resolved contract are
always present; margin is a bonus).

## Decisions (each tagged with provenance)

1. **Surface = a global `--preview` bool flag.** ✅ human-confirmed (2026-07-05) + 📖 code-verified
   (`GlobalOpts`, `src/cli.rs:20-43`, already carries `--live`/`--format`/`--md_type` as global; a
   flag reuses ALL existing order-arg parsing, a `preview` subcommand would duplicate 4 arg structs).
   Flows `GlobalOpts.preview → Config.preview` mirroring how `--live` collapses into `Config.port`
   (`src/config.rs:38`). Non-order commands ignore it (documented; only the six order verbs read it).
2. **Scope = all six order verbs**: `buy` `sell` `option-buy` `option-sell` `option-combo`
   `option-close`. ✅ human-confirmed + 📖 code-verified (all funnel through `place_with_client`,
   `trade.rs:317`; combo/close reach it too) — one branch at the choke point covers all, marginal
   cost ≈ 0 vs stock-only.
3. **Gate = identical to a real order** (fail-safe). ✅ human-confirmed. Preview reuses
   `require_live_write_gate(cfg)` (`trade.rs:143`: `port==LIVE_PORT && OMI_ALLOW_LIVE!=1 ⇒ reject`).
   Rationale: if Tiger *ignores* `what_if` it would **transmit a real order**; gating preview exactly
   like a real order guarantees preview is never *more* permissive than the thing it previews.
   `OMI_ALLOW_LIVE` is a session-level env (set once per trading session) → near-zero added friction.
   Relaxing to read-shaped is **out of scope**, deferred to a follow-up **only after** live-acceptance
   proves Tiger does not transmit (see §Risk register).
4. **`what_if` set on the built order, real transmit path byte-unchanged.** 📖 code-verified. Reuse
   `build_stk_order` / `build_option_order` / `build_combo_order` unchanged; when `cfg.preview`, set
   `order.what_if = true` before placement. The real path keeps `what_if=false`
   (`mod.rs:562` default) so the existing frozen stk/option/combo suites stay green.
5. **Uniform preview envelope, pure FROZEN shaper seam.** 📖 code-verified pattern (`trade.rs:51`
   already has a "Pure, FROZEN seam: the ack JSON — exact 6-key object"). Mirror it: a pure
   `shape_preview(...)` builds the exact envelope from already-extracted values (frozen); the
   `OrderState → margin/commission` extraction is a **gateway fn (review-by-reading; NOT frozen —
   needs a live gateway**, same class as every fn under `trade.rs:259`). ⚠️ assumed: the exact key set
   below (my design; arch may refine within the "uniform + stable" constraint).

   Envelope (stable keys):
   ```json
   {
     "preview": true,
     "what_if": true,
     "action": "BUY",
     "contract": { "…gateway-resolved contract identity…" },
     "order":    { "type": "LMT", "qty": 100, "limit": 250.0, "…echo…" },
     "margin":     { "init_change": <f64|null>, "maint_change": <f64|null>, "equity_with_loan_change": <f64|null> },
     "commission": { "value": <f64|null>, "min": <f64|null>, "max": <f64|null>, "currency": <str|""> },
     "warning": <str|"">,
     "status":  "<OrderState.status echo>"
   }
   ```
   Field sources (📖 code-verified, `ibapi OrderState`): `initial_margin_change` /
   `maintenance_margin_change` / `equity_with_loan_change` (all `Option<f64>`), `commission` /
   `minimum_commission` / `maximum_commission` (`Option<f64>`), `commission_currency` (`String`),
   `warning_text` (`String`), `status` (`OrderStatusKind`). `Option<f64>::None ⇒ JSON null` = the
   "ship value regardless of Tiger" behavior with zero special-casing.

## Success criteria (decision-complete)

1. Global `--preview` flag parses on the six order verbs; `omi <verb> … --preview` runs the whatIf
   path and **returns the uniform envelope; transmits nothing**.
2. Verbs covered: `buy` `sell` `option-buy` `option-sell` `option-combo` `option-close`.
3. Envelope keys are stable and uniform across all six verbs (§Decisions 5). Margin/commission fields
   are `null` when the gateway does not populate them (never absent, never a crash).
4. Preview honors `require_live_write_gate` exactly like a real order (no new relaxation).
5. Real transmit path unchanged: `what_if` stays `false` on non-preview placement; existing frozen
   stk / option-orders / option-combo / option-close suites remain GREEN.
6. **Freeze coverage** (recorded on the card): FROZEN = (a) `--preview` flag parse → dead-port
   connection envelope, black-box via `assert_cmd` mirroring the existing write-path parse tests;
   (b) `order.what_if == true` after the preview transform, `== false` on the real path; (c) the pure
   `shape_preview(...)` envelope shape/keys. REVIEW-BY-READING (gateway, not frozen): the
   `OrderState → margin/commission/warning` extraction + that whatIf does not transmit.
7. `cargo build` · `cargo clippy --all-targets -- -D warnings` · `cargo test` GREEN on the branch.
8. **Live-acceptance protocol** (operator, `:4001`, documented in the card): run
   `omi --live <verb> <sym> <qty> --limit <FAR-from-market> --preview` → envelope returns; then
   `omi --live orders` **shows NO resting order** ⇒ whatIf did not transmit **and** the Tiger-`what_if`
   premise is confirmed. A far-from-market LMT means even a refuted premise rests (never fills) and is
   visible/cancellable.

## Scope

- One `--preview` flag; the whatIf placement branch at the `place_with_client` choke point; the pure
  `shape_preview` seam + gateway extraction; the six order verbs; the uniform envelope.

## Non-scope (explicit)

- **NOT** order modify (cancel+replace already covers it), **NOT** stop/bracket/trailing/GTC/TIF
  variants, **NOT** any change to the real transmit path of live orders.
- **NOT** a server-side two-phase confirm — the human-confirm step lives in **hermes/TG**, the CLI
  stays stateless (preview and the real order are two independent calls hermes sequences).
- **NOT** relaxing preview to read-shaped (ungated) — deferred, evidence-gated (see §Risk register D1).
- **NOT** running the live-acceptance probe inside the pipeline (needs a live US-session gateway;
  operator-owned, same lifecycle precedent as write-path-semantics D2).

## Risk register

- **D1 — Tiger ignores `what_if` ⇒ preview transmits a real order.** ⚠️ the load-bearing premise.
  Mitigations in v1: (i) same write gate as a real order (Decision 3); (ii) live-acceptance uses a
  FAR-from-market LMT so a refuted premise rests without filling (criterion 8); (iii) `omi --live
  orders` after the probe is the confirm/refute observable. Refuted ⇒ open a fix feature (do NOT relax
  the gate). Confirmed ⇒ the read-shaped relaxation becomes a candidate follow-up.
- **D2 — margin/commission absent on Tiger even when it honors `what_if` (no-transmit but empty
  OrderState).** Handled by design: `Option<f64>::None → null`; the envelope's echo + resolved
  contract still make it a valid confirm card. No blocker.

## Handoff → pipeline-arch

Decide (against the codebase): exact placement of `preview` in `Config`/dispatch; the precise
`place_with_client` preview branch (new `preview_with_client` vs a `what_if: bool` param — keep the
real path byte-identical); final envelope key set within the "uniform + stable" constraint; where the
pure `shape_preview` seam lives; and the review-by-reading boundary for the OrderState extraction.
Emit `arch.md` + `CONTEXT.md` + any ADR. Do NOT touch `src`/`tests`.
