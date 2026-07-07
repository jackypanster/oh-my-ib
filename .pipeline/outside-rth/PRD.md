# PRD — outside-rth

Stage: prd · feature: outside-rth · repo: jackypanster/oh-my-ib · branch: main
Author: cc. Design grilled & locked via `/think` (2026-07-07); the two operator decisions are
recorded below (D1, D2). Extends the STK write path (ADR 0017 containment, ADR 0030/0031 gates)
with one opt-in order attribute. No new risk surface.

## Problem

`omi buy` / `omi sell` place STK orders that are eligible to fill **only during regular trading
hours (RTH)**. The IB TWS API can also admit an order for pre-market + post-market fills, but the
CLI has no way to request it: `build_stk_order` (`trade.rs:31`) leaves `Order.outside_rth` at its
`Default` (`false`), and nothing ever sets it. The operator wants to place limit orders that can
work the extended-hours sessions.

## Goal

Add an opt-in `--outside-rth` boolean flag to `buy`/`sell`. When set (and the order is LMT), the
placed `Order.outside_rth = true`, so the gateway admits the order to fill in RTH **and** pre-market
**and** post-market. Default (flag absent) = today's RTH-only behavior, byte-for-byte unchanged.

## API reality (verified against the ibapi 3.1 crate — the design's load-bearing fact)

- `ibapi::orders::Order.outside_rth: bool` exists (`orders/mod.rs:127`); doc string is IB's official
  text verbatim: *"If set to true, allows orders to also trigger or fill outside of regular trading
  hours."* Wired both ways over the wire: `proto/encoders.rs:163` sends it, `proto/decoders.rs:165`
  reads it back ⇒ the gateway honors it.
- **It is ONE boolean, not a pre/regular/post selector.** `true` = RTH + pre + post together; `false`
  = RTH only. IB has NO native pre-only / post-only flag; isolating a single session would require
  `good_after_time` + `good_till_date` (GTD) clock windows — explicitly out of scope (D5/non-scope).
- **MKT cannot fill outside RTH** — IB queues market orders to the next RTH open. `outside_rth` is
  only meaningful with a limit price; the limit also caps thin-liquidity extended-hours slippage.

## Decisions (provenance-tagged)

- **D1 — one boolean flag, NOT a 3-way session selector.** ✅ human-confirmed (`/think` 2026-07-07,
  AskUserQuestion). Maps 1:1 to IB's `outside_rth`. No emulation of pre-only/post-only (would need
  GTD windowing + timezone/half-day-holiday calendars — rejected as heavy, DST-fragile, and still
  unable to cleanly isolate pre vs post).

- **D2 — STK `buy`/`sell` ONLY in v1.** ✅ human-confirmed (`/think`, AskUserQuestion). Single-name
  US equity options are RTH-only, so the flag is a silent no-op on `option-*` verbs — exposing it
  there would mislead. Do NOT add `--outside-rth` to option/combo/close.

- **D3 — LMT-required: `--outside-rth` + MKT (no `--limit`) is a hard refuse.** ✅ human-confirmed
  (`/think`) · 📖 `ErrorKind::Config` doc is literally "Bad local config or **flag combination**"
  (`error.rs:16`) ⇒ the correct bucket. Refuse with `code="config"`, **exit 5**, message ~"outside-RTH
  requires a limit price (MKT orders do not fill outside regular trading hours)", offline / pre-connect.
  Rationale: MKT can't fill outside RTH anyway; the limit is also the slippage breaker.

- **D4 — NEW pure seam `apply_outside_rth(&mut Order, bool) -> Result<(), String>`; do NOT change
  `build_stk_order`'s signature.** ✅ code-survey-forced (mirrors live-combo-unlock D6). 📖
  `build_stk_order`'s 4-arg form is pinned by THREE existing frozen test files —
  `tests/stk_orders_command.rs:50,62`, `tests/order_preview_command.rs:45,76,84`,
  `tests/write_path_semantics_doc.rs:77,78`. Adding a 5th param breaks their compilation = editing
  another card's frozen spec-paths (hard invariant). So the flag is applied by a **new** seam that
  reads the already-set `order.order_type` (built by `build_stk_order`) for the D3 MKT guard and sets
  `order.outside_rth`. `place()` calls `build_stk_order` (unchanged) then `apply_outside_rth`.

- **D5 — guardrails UNTOUCHED; `outside_rth` is orthogonal to risk.** ✅ human-confirmed (`/think`).
  It changes *when* an order can fill, not risk size. The double live gate
  (`require_live_write_gate`), the ≤$500 notional cap (`check_live_write_posture`/`resolve_max_notional`),
  and the combo pure-width breaker (`combo_live_max_risk`) are all unchanged and stay in force. No
  risk ADR. On the live path the LMT-required guard (D3) is automatically satisfied (live is already
  LMT-only, ADR 0030).

- **D6 — preview echoes the flag, non-breakingly.** ✅ (`/think`) · 📖 `tests/order_preview_command.rs:52`
  asserts only shape_preview's **top-level** key set (8 keys) + `out["order"]["limit"]`, NOT the
  `"order"` sub-object's key set. So adding `"outside_rth": order.outside_rth` **inside** the nested
  `"order"` object leaves that frozen test green. `preview_stk_option` (`trade.rs:515`) forwards the
  same `order` built in `place()`, so once `apply_outside_rth` runs before `place_core`, both the
  `--preview` branch and the transmit branch carry the flag.

## Scope

- **IN** (`src/cli.rs`, `OrderArgs` `:106`): add `#[arg(long)] pub outside_rth: bool` (used by both
  `Buy`/`Sell`, which share `OrderArgs`).
- **IN** (`src/ib/trade.rs`): NEW pure seam `apply_outside_rth(order: &mut Order, outside_rth: bool)
  -> Result<(), String>` — `outside_rth && order.order_type == "MKT"` ⇒ `Err(reason)`; else
  `order.outside_rth = outside_rth; Ok(())`. Wire into `place()` (`:603`) right after
  `build_stk_order` (`:621`) and before `place_core`, mapping `Err` → `AppError::config(.., ctx)`.
  Ordering preserved: usage (qty/limit) < config (this guard + gate) < connection.
- **IN** (`src/ib/trade.rs`, `shape_preview` `:79`): add `"outside_rth": order.outside_rth` inside the
  existing `"order"` sub-object (D6; non-breaking to the frozen preview test).
- **IN** (`src/ib/mod.rs` `:45`): re-export `apply_outside_rth` alongside the other trade seams.
- **IN**: NEW frozen spec `tests/outside_rth.rs`.
- **OUT** (non-scope): 3-way session selector; `good_after_time`/`good_till_date` windowing;
  timezone/holiday calendars; options/combo/close verbs (D2); any change to `build_stk_order`'s
  signature (D4); any change to the live gate / notional cap / combo breaker (D5); new TIFs
  (GTC/GTD stay out); echoing `outside_rth` in the transmit ack (`shape_order_ack` stays 6-key —
  preview is the verification surface); modify/replace.

## Success criteria (acceptance)

1. **`apply_outside_rth` (offline, FROZEN):** build a LMT order (`build_stk_order(.., Some(px))`) +
   `apply_outside_rth(&mut o, true)` ⇒ `Ok(())` and `o.outside_rth == true`; `apply(false)` ⇒ `Ok`,
   `o.outside_rth == false`. Build a MKT order (`.., None`) + `apply(true)` ⇒ `Err(_)` (message names
   the limit requirement); `apply(false)` ⇒ `Ok`, `o.outside_rth == false`. [frozen]
2. **Preview echo (offline, FROZEN):** LMT order, `apply(true)`, `shape_preview(..)` ⇒
   `out["order"]["outside_rth"] == true`; default order ⇒ `== false`. Top-level key set unchanged.
   [frozen]
3. **CLI guard (black-box, FROZEN):** `omi --format json buy AAPL 1 --outside-rth` (no `--limit`) ⇒
   `code="config"`, exit 5, before any connect (works with no gateway). `omi --format json buy AAPL 1
   --limit 1 --outside-rth --host 127.0.0.1 --port 65000` ⇒ `code="connection"` (guard passed, past
   parse). [frozen]
4. **Default unchanged:** every existing frozen test stays GREEN — `build_stk_order` untouched
   (4-arg), `shape_preview` top-level keys unchanged, `stk_orders_command.rs` /
   `order_preview_command.rs` / `write_path_semantics_doc.rs` / `live_write_guardrail.rs` unmodified.
   [freeze gate + read]
5. **Guardrails intact:** `--outside-rth` does not touch the live gate, the notional cap, or the
   combo breaker; `--live` + LMT + cap semantics identical. [read]
6. **Paper acceptance (operator, `:4002`):** `omi buy AAPL 1 --limit 150 --outside-rth --preview` ⇒
   envelope shows `order.outside_rth: true`, `transmits: false`. A real place on `:4002` ⇒ order
   accepted; `omi orders` shows it working. [operator paper]
7. **Live acceptance (deferred, entitlement-gated):** a real post-market LMT on `:4001` fills or rests
   outside RTH. Same "code-ready, entitlement/session-gated" shape as the `[460]` combo — NOT part of
   the merge gate. [operator, deferred]
8. `cargo build` · full `cargo test` · `cargo clippy --all-targets -- -D warnings` green. [verify]

## Gotchas (project-specific traps the next nodes MUST know)

- **#1 frozen-seam collision — do NOT change `build_stk_order`'s arity.** Three frozen test files
  call it 4-arg (D4 lists them). The whole point of the `apply_outside_rth` seam is to avoid that
  break. Review's freeze gate + a full `cargo test` on the branch must confirm those files are
  untouched and green.
- **shape_preview: nest `outside_rth` INSIDE `"order"`, not at top level** — the frozen preview test
  asserts the exact 8 top-level keys (`order_preview_command.rs:52`). A top-level add falsely rejects.
- **Set the flag before `place_core`** so the shared `order` reaches BOTH the preview branch and the
  transmit branch (`preview_stk_option` forwards the same `order`).
- **Guard bucket = `config` (exit 5), not `usage`** — it is a flag *combination* error (D3), matching
  `ErrorKind::Config`'s doc and the live-gate precedent; keep it offline/pre-connect.
- **STK-only** (D2) — resist the urge to thread the flag through the option verbs "for consistency";
  it is a no-op there and misleads.
- Most fragile assumption: Tiger's gateway/account actually *routes* extended-hours fills. If not, the
  flag is a harmless no-op (order waits for RTH). We prove SENT+accepted (preview + paper `omi orders`);
  a real pre/post fill is deferred acceptance (criterion 7), not a merge blocker.

## Verify

`cargo build` · `cargo test` (new `tests/outside_rth.rs` red→green; all prior suites green) ·
`cargo clippy --all-targets -- -D warnings`. Operator paper: criterion 6 on `:4002`. No gateway needed
for the frozen offline checks (seam + guard + preview echo).

## For arch (next stage)

- Decide whether a lightweight ADR (0032) records the outside-RTH attribute + LMT-guard + STK-only
  decisions for audit-trail parity with prior features (recommend yes, short).
- Confirm the seam name/signature (`apply_outside_rth`) and the exact refuse message wording.
- Confirm `CONTEXT.md` needs no new glossary term beyond "outside-RTH / extended hours = pre + post".
