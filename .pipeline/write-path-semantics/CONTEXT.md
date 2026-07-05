# CONTEXT — write-path-semantics

Delta on the prior CONTEXT chain (order-account-stamp → …). New terms:

- **Reference-behavior table** — the 7-column map (`field → gateway semantics → verification tier`)
  that PROVES understanding of the Tiger/IB field reference before we trust our own construction
  (Reference-Port technique). Lives at `docs/write-path-semantics.md`.
- **Verification tier** — `✅ paper-probe` (date + paper account) | `📖 doc-cite` (IB TWS API doc /
  ibapi source line) | `⚠️ UNVERIFIED`. `⚠️` = the risk register; it **ships** (never a merge blocker,
  D2).
- **Load-bearing default** — an `ibapi::Order::default()` field whose value is non-inert OR flips
  behavior (`transmit=true`, `outside_rth=false`, `display_size=Some(0)`, `what_if=false`,
  `origin=Customer`, `exempt_code=-1`), silently sent via `..Default::default()`. Contrast the **inert
  tail** (~70 empty/zero/`None`/`vec![]` fields → one catch-all row).
- **Default-canary** — the frozen assertion that ibapi's load-bearing defaults equal the doc's claims;
  fires on an upstream version bump that flips one (the `transmit=false` catastrophe guard).
- **Probe recipe** — per `⚠️` row: the exact `omi` command on `:4002` + the observable that
  confirms/refutes + the fallback. Execution **deferred** to a live US session (option-close lifecycle
  precedent).
- **Anti-rot source-scan** — the frozen test scans `src/ib/trade.rs` (the ONLY write file) for every
  field set and asserts each is documented, so the doc cannot silently fall behind the code.

## Conventions (feature-specific)

- **Doc-only**: the product is Markdown under `docs/`, guarded by `tests/write_path_semantics_doc.rs`.
  No `src/` change (D6) — a wrong value found is REGISTERED (`⚠️`), fixed in a separate feature.
- **Freeze uses runtime `read_to_string`, never `include_str!`** — an absent doc must FAIL the test, not
  fail to COMPILE (CONTRACT: spec must compile and fail).
- Field inventory is bound to **ibapi 3.1.0** (`orders/mod.rs:478`, `contracts/builders.rs`).
- **SKILL-PROPOSAL (not this feature)**: make a reference-behavior subsection REQUIRED for write cards,
  via `pipeline-improve` against `jackypanster/pipeline`.
