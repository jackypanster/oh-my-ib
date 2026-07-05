# arch — preview-readonly (fix: `--preview` never transmits)

Status: arch (2026-07-05, feature preview-readonly). Consumes PRD.md. Binding decisions → ADR 0027.
All PRD decisions are `✅ human-confirmed` or `📖 code-verified`; no arch-level human grill needed. R1 is
now KNOWN (refuted, ✅ probed) — the new design removes the whatIf dependency entirely, so there is no
unverified external premise (unlike order-preview). CONTEXT.md records R1=refuted + the new posture.

## Chosen shape

Replace the transmitting preview (`preview_with_client` = `place_order(what_if=true)`) with a **read-only**
path: resolve the contract via `client.contract_details` (STK / single-leg option) or reuse the
already-resolved conids (combo / close), echo the built order, compute notional, and shape an envelope
with **`transmits:false`**. The preview branch moves **BEFORE** `require_live_write_gate` (read-shaped).

## Decisions

1. **Module = keep in `src/ib/trade.rs`** (PRD Decision 6 resolved, 📖 code-verified). Minimal churn
   (modify the existing preview code in place), reuses the pure builders (`build_stk_order` etc.) already
   there, and the containment invariant ("`place_order`/`cancel_order` ONLY in trade.rs", AGENTS.md) is
   **unaffected** — a read-only preview adds no write calls. Extraction to `preview.rs` is deferred
   cleanup, out of scope. → ADR 0027.
2. **Read-shaped gate** (✅ human-confirmed): the preview branch runs before `require_live_write_gate`, so
   `--preview` needs `--live` only (no `OMI_ALLOW_LIVE`), like `omi contract`/`quote`.
3. **Full read-only envelope** (✅ human-confirmed) — see below. Drops `what_if`/`margin`/`commission`/
   `status`; adds `transmits:false` + `notional`.
4. **All six verbs**; **real order path byte-unchanged**; `place_order`/`cancel_order` stay ONLY on the
   real path.

## Component changes (all in `src/ib/trade.rs`)

| current | change |
|---|---|
| `shape_preview(&Contract,&Order,&OrderState)` (:77) | → `shape_preview(contract_json, &Order, notional, ccy)` — pure FROZEN, builds the read-only envelope from an already-resolved **contract JSON** + order echo + notional. No `OrderState`. |
| `preview_with_client` (:423, `place_order`+`what_if`) | **REMOVE**. Replace with `preview_stk_option(&Client, ctx, &Contract, &Order)` — `client.contract_details(&contract)` (READ) → resolved contract JSON → `shape_preview`. NO `place_order`. |
| `place_core` (:471) gate→connect→branch | restructure: `if cfg.preview { connect → preview_stk_option } else { require_live_write_gate → connect → resolve_account → place_with_client }`. Preview: no gate, no account. |
| `option_combo` preview branch (:820) | preview → shape from the ALREADY-resolved `legs_snapshot` (leg specs + conids) + net notional. No extra gateway call, no gate. |
| `option_close` preview branch (:1072) | preview → shape from the ALREADY-resolved held contract (conid known) + notional. No extra gateway call, no gate. |

## Read-only envelope (stable keys, `transmits:false`)

```json
{ "preview": true, "transmits": false, "action": "BUY",
  "contract": { "symbol": "AAPL", "conid": 265598, "sec_type": "STK", "exchange": "SMART",
                "currency": "USD", "long_name": "APPLE INC"
                /* option adds: expiry, strike, right; combo: "legs":[{action,ratio,conid,...}] */ },
  "order":    { "type": "LMT", "qty": 100, "limit": 290.0 },
  "notional": 29000.0, "notional_currency": "USD",
  "note": "margin/commission unavailable on this gateway (whatIf transmits)" }
```

- **notional** = `qty × limit × multiplier`: STK ×1; single-leg OPT ×100; combo ×`|net_limit|`×100;
  close ×100 (📖 option multiplier "100", `option_quote.rs:87`).
- `contract` sub-object varies by instrument (STK / OPT / BAG-legs); top-level keys are uniform.

## Freeze boundary (arch → task)

- **FROZEN** (task writes red tests over `spec-paths` — REPLACES the old `order_preview_command.rs`
  assertions): the pure `shape_preview` read-only envelope (exact keys incl. `transmits:false`, `note`,
  no `what_if`/`margin`); notional math (STK ×1, OPT ×100); and the **read-shaped gate** black-box —
  `omi <verb> … --live --preview` WITHOUT `OMI_ALLOW_LIVE` on a dead port ⇒ **`connection`** (past the
  gate), NOT `config`. This FLIPS the old frozen assertions (`what_if:true`; gate=`config`).
- **REVIEW-BY-READING** (needs a live gateway): the `contract_details` wiring; and **containment** — the
  preview path calls NO `place_order` (grep: `place_order`/`cancel_order` appear only on the real path).
- **Live-acceptance** (cc runs omi, operator directive — Hermes/TG deferred): `omi --live buy AAPL 1
  --limit 1 --preview` returns the resolved envelope AND `omi --live orders` is EMPTY ⇒ R1 fixed.

## Non-scope

margin/commission (impossible read-only on Tiger); real order path; Hermes/TG wiring (deferred); the old
whatIf envelope (dropped — breaking change, acceptable).
