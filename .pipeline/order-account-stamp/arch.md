# Architecture — order-account-stamp

One card, ~15 lines of change + one pure seam + one new spec file. Everything rides the
existing placement machinery.

## Code-first verification results

1. **No frozen test asserts `order.account`** (grepped stk/option/combo suites) — the
   builders' pure output (`account: ""` via Default) stays valid; the stamp is
   gateway-path only.
2. **Exactly one `.place_order(` call site** (`place_with_client`) — the choke point
   already exists; enforcement = making its account parameter REQUIRED.
3. **`Order` derives `Clone`** (ibapi) — the choke point can clone+stamp without changing
   its `&Order` intake, keeping all callers' borrow patterns.
4. **`--account` is a global flag** (cli.rs:39) flowing into `cfg.account`;
   `resolve_account` (mod.rs:99) is the single account authority; `AccountId.0` access has
   the positions.rs precedent.

## Design (pins PRD D1-D4)

### Pure FROZEN seam

```rust
/// Stamp the resolved account onto an order (ADR 0024). Sets `order.account`
/// (OVERWRITING any prior value — the resolved account is the only authority)
/// and touches nothing else.
pub fn stamp_order_account(order: &mut Order, account: &str)
```

### Choke point (signature change, gateway path — not frozen)

```rust
fn place_with_client(
    client: &Client,
    ctx: &str,
    contract: &Contract,
    order: &Order,
    account: &AccountId,   // NEW, required — no caller can skip the stamp
    ack: impl Fn(i32, &str) -> Value,
) -> Result<Value, AppError>
// body: let mut order = order.clone(); stamp_order_account(&mut order, &account.0);
//       ...existing allocate → place_order(&order) → bounded first-ack UNCHANGED.
```

Callers (all three, per PRD D4):
- `place_core` (stk + single-leg option): after its existing `connect`, add
  `let account = super::resolve_account(&client, cfg)?;` and pass it through.
- `option_combo`: same — resolve after connect (before per-leg conid resolution is fine;
  one bounded call), pass through.
- `option_close`: pass its ALREADY-resolved `account` (no second `managed_accounts`).

### AGENTS.md amendment

Phase-2 orders sentence gains: "orders are stamped with the resolved account
(`--account` honored on writes)". CLAUDE.md untouched.

## ADR 0024 records

Choke-point enforcement (vs per-verb stamping) · overwrite semantics · frozen builders
untouched (stamp post-build) · Tiger-accepts-explicit-account assumption with fallback
(stamp only when `cfg.account` set) — fallback needs operator ack, not auto-applied.

## Freeze plan handed to task (advisory)

- `tests/order_account_stamp.rs` (NEW, only spec surface): seam matrix — stamps empty
  account; OVERWRITES pre-set value; all other fields byte-identical before/after for an
  LMT order (limit_price Some) AND an MKT order (limit_price None); empty-string account
  stamps as empty (degenerate totality, no panic).
- spec-paths = that file; impl-paths = src/ib/trade.rs, src/ib/mod.rs, AGENTS.md.
- Card verify: `cargo test --test order_account_stamp`; full-verify unchanged.
