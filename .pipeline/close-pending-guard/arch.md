# Architecture — close-pending-guard

One card on the verified option-close machinery. The guard is ~30 lines in `trade.rs`
(one pure FROZEN seam + wiring) + one new spec file. Everything else VERBATIM reuse.

## Code-first verification results (all checked in-repo at prd/arch time)

1. **Insertion point exists and is unambiguous**: `option_close` in `src/ib/trade.rs` —
   after the non-OPT `usage` check, before `derive_close`. The client is already connected
   (step 3 of ADR 0022 flow); the guard adds ZERO connects.
2. **Drain reuse is free**: `open_orders_with_client(client, account_filter, ctx)` in
   `src/ib/orders.rs` is `pub(crate)`, takes `&Client`, returns `Value::Array` of rows
   already carrying `order_id`, `conid`, `action` ("Buy"/"Sell" Debug strings). Shared with
   `brief` (ADR 0010/0011 precedent for cross-module reuse). Live-verified read path.
3. **No status filtering needed**: the drain matches only `Orders::OrderData` items from
   `all_open_orders()` — working orders by definition (terminal orders are the
   completed-orders domain). PendingCancel still blocks = fail-closed correct.
4. **Frozen-spec interplay**: `derive_close`/`shape_option_close_ack` signatures and the
   whole `tests/option_close_command.rs` + `tests/positions_row.rs` surfaces are untouched
   — the guard refuses BEFORE `derive_close` executes, and only on the gateway path
   (not offline-reachable), so no existing frozen test can observe it.

## Component design (card 01, all in src/ib/trade.rs)

### Pure FROZEN seam

```rust
/// Working orders that BLOCK a close on `conid` for a position of this sign:
/// same conid AND action opposite to the position (long ⇒ "Sell" blocks,
/// short ⇒ "Buy" blocks). Same-side orders never block. Returns ids ASCENDING.
/// position == 0.0 ⇒ empty (unreachable in the verb — flat is refused upstream).
pub fn blocking_close_order_ids(
    position: f64,
    conid: i32,
    open_orders: &[(i32, i32, String)], // (order_id, conid, action "Buy"/"Sell")
) -> Vec<i32>
```

Semantics table (the frozen matrix):

| position | order (conid match) | action | blocks? |
|---|---|---|---|
| +2 (long) | same | Sell | YES |
| +2 | same | Buy | no (add, not close) |
| -2 (short) | same | Buy | YES |
| -2 | same | Sell | no |
| any | OTHER conid | any | no |
| 0 | same | any | no (defensive totality) |

Multiple blockers ⇒ all ids, sorted ascending (deterministic output for the frozen test
and for the agent-facing message).

### Guard wiring (gateway path, review-by-reading)

```
option_close step 5.5 (new; between non-OPT check and derive_close):
  rows = super::orders::open_orders_with_client(&client, None, ctx)?   // D1/D3: no account filter
  triples = rows.as_array() → map (order_id as i32, conid as i32, action as String)
            — missing/malformed fields in a row ⇒ data error naming the row index
            (never silently skip: a skipped row could hide a blocker — fail-closed)
  ids = blocking_close_order_ids(row.position, args.conid, &triples)
  if !ids.is_empty() ⇒ Err(not_found):
    "close blocked: working close order(s) [<ids comma-joined>] already cover conid <N> —
     cancel first (`omi cancel <id>`) or inspect `omi orders`; a second close would flip
     the position"
```

Ordering becomes: usage < config < connection < position-match(not_found) <
non-OPT(usage) < **pending-guard(not_found)** < derive < rebuild < conid-assert < place.

### AGENTS.md amendment (docs ride the PR)

Phase-2 option line, option-close phrase gains: "; refuses while a working close order
exists on the conid (anti double-fire)". **CLAUDE.md untouched** (byte budget not spent —
verb list unchanged).

## ADR

ADR 0023 records: brute-force refuse (vs qty-budget) · no status filter · no account
filter · not_found reuse (vs new error code) · TOCTOU residual accepted (fixed client_id
de-facto mutex) · fail-closed row-parse (malformed drain row ⇒ error, never skip).

## Freeze plan handed to task (advisory)

- `tests/close_pending_guard.rs` (NEW, the feature's ONLY spec surface): the seam matrix
  above (~7 asserts incl. multi-blocker ordering + empty cases). Pure fn — no binary
  invocation, no gateway, no gate tests (already frozen elsewhere).
- spec-paths = that one file; impl-paths = src/ib/trade.rs, src/ib/mod.rs (re-export),
  AGENTS.md. Card verify: `cargo test --test close_pending_guard`.
- full-verify stays `[cargo build, cargo test]`.
