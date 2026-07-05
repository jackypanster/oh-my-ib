# PRD — write-path-semantics (reference-behavior audit)

Status: prd (2026-07-05, feature write-path-semantics)
Origin: `/think` on the "Reference-Port" case → one-time retroactive audit of the SHIPPED write path.
Technique root: *"prove understanding of the reference implementation in a reviewable finished form
before porting any code."* Here the "reference" is not another language — it is the **Tiger/IB
gateway's field semantics** for the `(Contract, Order)` the write path constructs.

## Problem

- `src/ib/trade.rs` builds `(Contract, Order)` sent to the Tiger/IB gateway. The correctness of
  every field is defined by **external IB TWS API semantics** — code we do not own and cannot see in
  the diff. `ibapi::orders::Order` has **~90 fields** (`ibapi-3.1.0/src/orders/mod.rs:73`).
- **Frozen tests can't reach the gateway.** They assert only pure builder output — e.g.
  `assert_eq!(order.order_type, "LMT")` (`tests/option_orders_command.rs:65`). Gateway fns are
  explicitly `// review-by-reading; NOT frozen — needs a live gateway` (`trade.rs:259`). So the suite
  freezes our *understanding* of gateway semantics, **not** the semantics. A misunderstanding is
  locked in by the very test meant to protect it — the silent-failure the Reference-Port case warns of.
- **This already shipped a real, money-critical bug.** `--account` was silently NOT honored on writes
  until audit #2 → ADR 0024 (order-account-stamp). A field-semantics assumption was wrong and lived on
  trunk placing real orders to the default account.
- **The habit exists but is ad hoc.** ADR 0024 §5 already writes a lightweight version (assumption +
  paper-probe + fallback), but only **2 of 6** write ADRs (0022, 0024) record any external-behavior
  assumption; **0017/0018/0020/0021 record none**. No consolidated write-path semantics doc exists.
- **New, harder evidence found this stage (`transmit`).** Every order is built with
  `..Default::default()`, which pulls ibapi's **custom** `impl Default for Order`
  (`mod.rs:478`) — NOT a derived Default. That custom impl sets **`transmit: true`** (`mod.rs:494`).
  Under a *derived* Default, `transmit` would be `false` and **no order would ever be sent** (staged
  at TWS only). The write path's most basic property — "orders actually transmit" — silently depends
  on an upstream crate default that **nothing in oh-my-ib tests or documents**. Same class:
  `outside_rth: false`, `display_size: Some(0)` (the crate itself carries `// TODO - default to None?`),
  `what_if: false`, `origin: Customer`, `exempt_code: -1`.

## Goal

Produce **one** durable, reviewable, agent-first doc — `docs/write-path-semantics.md` — that maps every
field the write path sends to the gateway (explicitly-set **and** load-bearing inherited defaults) to
its reference semantics, our chosen value, deliberate divergences, boundary cases, and a **verification
tier**. The `⚠️ UNVERIFIED` rows form a **risk register**; each carries a runnable paper-probe recipe.
One-time retroactive audit of the CURRENT shipped write path — the killer app: it audits assumptions
that are placing REAL orders today.

## Success criteria (decision-complete)

1. `docs/write-path-semantics.md` exists — agent-first (dense, structured, machine-parseable; AGENTS.md
   §Authoring).
2. **Coverage** = every `Order`/`Contract` field the write path SETS explicitly across
   `build_stk_order` / `build_option_order` / `build_combo_order` / `stamp_order_account` / the cancel
   path, **plus** every load-bearing defaulted field (value non-inert OR behavior-flipping) inherited
   from `ibapi::Order::default()`. The inert tail (~70 empty/zero/`None`/`vec![]` fields) is covered by
   **ONE catch-all row** ("all remaining fields inherit ibapi Default = inert empty/zero; not
   send-meaningful"). Target ~15–20 rows.
3. Each row columns: `field | our value | ibapi type/behavior | Tiger/IB reference semantics | why this
   value / deliberate divergence | boundary cases | verification tier`.
4. **Verification tier** ∈ { `✅ paper-probe` (with date + paper account) | `📖 doc-cite` (IB TWS API
   doc section/URL or ibapi source line) | `⚠️ UNVERIFIED` }. The doc **SHIPS with `⚠️` rows present** —
   they are the risk register, **not** a merge blocker (D2).
5. A **`## ⚠️ Risk register`** section lists every `⚠️` row with a concrete runnable **probe recipe**:
   the exact `omi` command on `:4002`, the observable that confirms/refutes, and the fallback if
   refuted — mirroring ADR 0024 §5 + the order-account-stamp probe (journal seq=4).
6. Pre-filled known-resolved rows: **`Order.account` = ✅** (order-account-stamp paper probe, journal
   seq=4/6 — Tiger accepts an explicit `Order.account`; no fallback needed).
7. **`transmit: true` is an explicit load-bearing row** (reference: `ibapi mod.rs:494` custom Default;
   risk: an upstream derive/upgrade flipping it silently stages orders without sending).

## Scope

- One-time retroactive audit of the CURRENT shipped write path: stk `buy`/`sell`, `option-buy`/`sell`,
  `option-combo`, `option-close`, `cancel`.
- Output: the single doc + its `⚠️` risk register with probe recipes.

## Non-scope (explicit)

- **NOT running the paper probes.** Executing them needs a live US-session gateway → deferred operator
  lifecycle (option-close fill-lifecycle precedent). Feature lands as the offline doc + recipes (D2).
- **NOT changing any write-path code** / builders / order construction. Pure documentation/audit. If the
  audit finds an actual wrong value (not merely unverified), it is **registered here, fixed in a
  separate feature** (D6) — this one only surfaces & registers.
- **NOT the read path.**
- **NOT a pipeline process/skill change.** Making the reference-behavior table a REQUIRED arch/ADR
  subsection (the `/think` "Step 2") is a **pipeline-skill** change → carried as a `SKILL-PROPOSAL`
  (see handoff), handled via `pipeline-improve` against `jackypanster/pipeline`, NOT this feature's PR.

## Resolved decisions

- **D1** Scope = Step 1 only (audit doc + register); Step 2 (process) → `SKILL-PROPOSAL`. [operator]
- **D2** Merge ships the doc WITH the `⚠️` register; probes deferred to a live session. [operator]
- **D3** Home = one durable repo doc `docs/write-path-semantics.md`, agent-first, cross-verb. [operator]
- **D4** Depth = explicit fields + load-bearing defaults; inert tail = one catch-all row (~15–20). [operator]
- **D5** Verification tiers = `✅` / `📖` / `⚠️`; `⚠️` ships as the risk register, never a blocker. [think]
- **D6** An actual wrong value found by the audit is REGISTERED here, fixed in a separate feature. [scope guard]

## Notes for arch (arch's call — NOT decisions)

- **Freeze-ability is the top design question.** A reference-behavior doc has no conventional red-test
  surface. Candidate frozen spec worth weighing: a **coverage test** that parses `src/ib/trade.rs` for
  every builder field set (`order.<field> =`, struct-literal fields, `stamp_order_account`) and asserts
  each appears as a row in `docs/write-path-semantics.md` — turning the doc into a MAINTAINED artifact
  that fails when a future write field is added without a semantics row. Arch decides: build this, or
  rely on review-by-reading (CONTRACT §Freeze coverage). This is the highest-leverage arch choice.
- **Field-inventory source of truth:** `ibapi-3.1.0/src/orders/mod.rs:73` (struct) + `:478` (custom
  Default). Contract defaults via `Contract::stock/call/put/spread` builders (SMART / USD /
  multiplier-100).
- **`📖` tier sources:** IB TWS API official docs (Order/Contract field reference) + ibapi source lines.
- **Money-critical unverified candidates already spotted** (seed the `⚠️` register): combo
  `limit_price` sign-free "negative = credit" (ADR 0021, no recorded verification); `transmit: true`
  dependency; `outside_rth: false`; `display_size: Some(0)` (crate TODO); TIF=Day; `what_if: false`.
