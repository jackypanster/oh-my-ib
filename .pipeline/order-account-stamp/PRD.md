# PRD — order-account-stamp (audit finding #2 fix)

Feature: every placed order carries the RESOLVED account (`Order.account`), so `--account`/
config account governs writes exactly as it already governs reads. ONE card, ~15 lines +
one pure seam. Safety machinery untouched (ADR 0017/0018/0022/0023 as-is).
Status: decision-complete (2026-07-04 audit /think + operator "开干"; full-auto).

## Problem

All orders are built with `..Default::default()` ⇒ `Order.account == ""` ⇒ the gateway
routes writes to its DEFAULT account. Reads honor the resolved account
(`resolve_account`: cfg/--account, else first managed account); writes silently ignore it.
Latent single-account today; catastrophic on a multi-account session: `option-close`
matches the position in account A, the order lands in default account B — the anti-open
gate is defeated cross-account and B receives a naked short. Audit finding #2.

## Goal

Stamp the resolved account onto the order AFTER connect, BEFORE `place_order`, enforced at
the SINGLE shared placement choke point (`place_with_client`) so no current or future verb
can skip it. `cancel` untouched (order-id domain is account-agnostic).

## Success criteria (acceptance)

1. Pure seam (FROZEN, offline — `tests/order_account_stamp.rs`):
   `stamp_order_account(&mut Order, &str)` sets `order.account` (including OVERWRITING a
   pre-existing value) and touches NOTHING else — every other Order field asserted
   byte-identical before/after (LMT and MKT shapes both covered).
2. Choke point (review-by-reading): the stamp executes inside `place_with_client` for ALL
   placement paths — stk buy/sell + option-buy/sell (via `place_core`), `option_combo`,
   `option_close`. Zero placement calls bypass it (grep: exactly one `.place_order(`
   call site, already true — the stamp sits on its path).
3. Account authority = `resolve_account` VERBATIM (cfg/--account else first managed) —
   the same value reads use; `option_close` reuses its already-resolved account (no second
   `managed_accounts` round trip on that path).
4. Existing frozen suites BYTE-UNTOUCHED and green (stk/option/combo builder suites assert
   pure-builder output which still has `account=""` — the stamp is gateway-path only;
   verified: no frozen test asserts `order.account`).
5. `cargo build` · clippy `-D warnings` · full `cargo test` green.
6. AGENTS.md Phase-2 line gains "orders are stamped with the resolved account
   (`--account` honored on writes)". CLAUDE.md UNTOUCHED.
7. Merge gate (paper, RUNNABLE TODAY — acks need no fills): far-off `option-buy`
   ⇒ PreSubmitted; `omi orders` row shows `account == DUQ653733` (the resolved paper
   account); `omi cancel` ⇒ Cancelled. A gateway REJECTION of an explicitly-set account
   is the fragile-assumption trigger (below) — observation + operator decision, not an
   impl defect.

## Scope

- `src/ib/trade.rs`: pure FROZEN seam `stamp_order_account`; `place_with_client` gains the
  account (resolution or value — arch pins the exact signature); `place_core`/
  `option_combo`/`option_close` call sites adjusted.
- `src/ib/mod.rs`: re-export the seam. `tests/order_account_stamp.rs`: NEW spec file.
- AGENTS.md one phrase. Nothing else (no CLI change — `--account` already global, cli.rs:39).

## Non-scope

- `cancel` (account-agnostic by protocol). No multi-account acceptance (single-account
  env; the fix makes the routing EXPLICIT, which is the whole point). No read-path change.

## Resolved decisions (locked)

- D1 **Choke-point enforcement**: stamp inside `place_with_client`, not at each verb —
  future verbs inherit it for free; review reduces to one site.
- D2 **Overwrite semantics**: the resolved account ALWAYS wins (deterministic; resolved =
  the authority reads already trust). No "respect pre-set value" arm.
- D3 **Frozen builders untouched**: signatures and output (account="") unchanged — the
  three existing order suites stay valid; the stamp is post-build, gateway-path.
- D4 **option_close passes its existing resolved account** — no duplicate
  `managed_accounts` call (one bounded read saved; same value by construction).

## Risks / fragile assumptions

- **Tiger accepts an explicitly-set `Order.account`** (some gateways reject unknown/
  malformed values). Paper-probeable TODAY (criterion 7, no fills needed). Rejection ⇒
  journaled observation + operator decision (likely fallback: stamp only when
  cfg.account is explicitly set). This is THE assumption; everything else is mechanical.
- Rollback: one revert removes the stamp; no schema/CLI change.

## Verification

- Offline frozen: seam matrix (criterion 1). Review-by-reading: choke-point wiring,
  option_close reuse, frozen suites untouched. Live paper: criterion 7 (today).
