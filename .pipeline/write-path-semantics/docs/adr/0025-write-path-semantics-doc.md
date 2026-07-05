# ADR 0025 — write-path-semantics: a test-guarded living reference doc

Status: accepted (2026-07-05, feature write-path-semantics)
Context: `/think` on the Reference-Port case. The write path builds `(Contract, Order)` whose field
semantics are defined by the **external Tiger/IB gateway** — a reference nothing in-repo tests. Frozen
builder tests assert pure construction (`order_type=="LMT"`, `option_orders_command.rs:65`), not gateway
meaning; gateway fns are unfrozen (`trade.rs:259`). A wrong field-semantics assumption already shipped
(`--account`, ADR 0024). `transmit:true` is silently inherited from ibapi's **custom** `Default`
(`mod.rs:494`) — a *derived* Default would be `false` ⇒ orders staged-not-sent — untested, undocumented.

## Decision

1. **Deliverable = ONE durable agent-first doc** `docs/write-path-semantics.md`: a **7-column**
   reference-behavior table — `field | our value | ibapi type/behavior | Tiger/IB reference semantics |
   why this value / deliberate divergence | boundary cases | verification tier` — covering every
   explicitly-set `Order`/`Contract` field + every load-bearing inherited default; the inert tail (~70
   empty/zero/`None`/`vec![]` fields) is **one catch-all row**. Plus a `## ⚠️ Risk register` with a
   runnable probe recipe per unverified row. ~15–20 rows.

2. **Verification tiers**: `✅ paper-probe` (date + paper account) / `📖 doc-cite` (IB TWS API doc
   section/URL or ibapi source line) / `⚠️ UNVERIFIED`. The doc **SHIPS with `⚠️` rows** — they are the
   risk register, **not** a merge blocker (D2). Running probes is a **deferred live-session lifecycle**
   (option-close fill-lifecycle precedent), not part of this feature's merge gate.

3. **FREEZE = a coverage + default-canary test, NOT review-by-reading alone.** REJECTED review-only:
   the doc would rot silently and the `transmit`-catastrophe guard would never exist. The frozen test
   (task-owned, `tests/write_path_semantics_doc.rs`):
   - **(a)** `read_to_string(docs/write-path-semantics.md)` is `Ok` — **RED now** (doc absent), GREEN
     when written. **GOTCHA: use runtime `std::fs::read_to_string`, NOT `include_str!`** — `include_str!`
     on an absent file fails to **COMPILE**, violating CONTRACT's "spec must compile and FAIL".
   - **(b)** every **required field token** has a table row carrying one tier marker (`✅|📖|⚠️`).
   - **(c)** an **anti-rot source-scan** of `src/ib/trade.rs` (the ONLY write file, AGENTS.md) extracts
     every field the code sets (`order.\w+ =` assignments + `Order { … }` struct-literal fields) and
     asserts each is a documented row — a new write field ⇒ test fails until documented.
   - **(d)** **default-canary**: `ibapi::Order::default()` load-bearing values (`transmit==true`,
     `outside_rth==false`, `what_if==false`, `tif==Day`, `display_size==Some(0)`, `origin==Customer`,
     `exempt_code==-1`) equal what the doc claims — GREEN now, **fires on an ibapi upgrade that silently
     flips a default**. This is the `transmit` guard, enforced.

4. **Freeze coverage (review reads, NOT frozen):** the SEMANTIC TRUTH of each row's reference-semantics
   / boundary columns is not machine-checkable offline — review-by-reading + the deferred paper probes
   resolve it. The frozen test guarantees **structural coverage + upstream-default pinning only**
   (CONTRACT §Freeze coverage).

5. **Pure audit — NO change to `src/ib/trade.rs` order construction (D6).** A wrong value found ⇒
   registered in `⚠️`, fixed as a SEPARATE feature.

6. **Process change is OUT (D1):** making a reference-behavior subsection REQUIRED for write cards is a
   pipeline-skill change → `SKILL-PROPOSAL` via `pipeline-improve` against `jackypanster/pipeline`, not
   this feature's PR.

## Consequences

- `docs/write-path-semantics.md` becomes a **living artifact**: CI-red when a write field is added
  without a semantics row (c), or when an upstream default drifts (d).
- The `transmit:true` dependency and the combo sign-free-credit claim (`trade.rs:577`, ADR 0021, no
  recorded verification) get a first-class home + a deferred probe recipe.
- One new test file (spec) + one new doc (impl). No product-code change; the 220-test suite is untouched
  except the added file (which is RED until the doc lands, per CONTRACT — one feature in flight, no CI).
- Field inventory is bound to **ibapi 3.1.0** (`mod.rs:478` Default; `contracts/builders.rs` SMART/USD/
  multiplier-100); the canary surfaces any change on a version bump.
