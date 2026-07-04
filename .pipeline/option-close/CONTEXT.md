# CONTEXT — option-close

Delta on `.pipeline/option-orders/CONTEXT.md` (option write-path domain). New/changed terms:

- **Close** — flatten an existing option position by placing the OPPOSITE-side order on the
  SAME contract. The TWS protocol has no close endpoint; US options are fungible (no
  open/close flag). `option-close` is a convenience+safety layer, not a new API.
- **Anti-open gate** — a close can only target a conid CURRENTLY HELD (matched from the
  live portfolio stream). Not held / already flat ⇒ `not_found`, nothing placed. Kills the
  failure mode "identity typo opens a NEW position".
- **Anti-double gate** — the order side is DERIVED from the held position's sign
  (long ⇒ SELL, short ⇒ BUY), never user-supplied. Kills "side inversion doubles the
  position". Over-close (`--qty` > |position|) is rejected — a close never flips a position.
- **Wrong-contract gate** — the placement contract is REBUILT from the matched row's
  decoded identity (proven builder chain), then `contract_details`-asserted:
  first-row conid must equal `--conid`, else `data` error BEFORE any order.
- **Position identity** — `positions`/`brief` rows carry `sec_type` (IB wire code via
  `SecurityType` Display: `"STK"`, `"OPT"`) + `expiry`/`strike`/`right`/`multiplier`
  (populated iff OPT, else `null`). Canonical sec_type form = wire code; the `contract`
  command's Debug-format (`"Stock"`) is a known legacy inconsistency, NOT changed here.
- **Full close (default)** — omitted `--qty` closes |position|; partial close is whole
  contracts `>= 1` and `<= |position|`.
- **Close ack** — exact 10 keys: `order_id, status, conid, symbol, expiry, strike, right,
  action, quantity, limit_price` — identity ECHOES THE MATCHED ROW (resolved truth), not
  user input; `action` is the derived side.

## Conventions (feature-specific)

- Validation ordering FROZEN: usage < config (live gate) < connection; runtime
  `not_found`/`usage`(non-OPT)/`data` require a gateway (review-by-reading + paper
  acceptance).
- Single-connect invariant: ONE client for drain + resolve-assert + place (option-combo
  review lesson — a second same-client-id connect wedges the gateway).
- Row-shape freeze is NEW in card 01 (`position_row` was `pub(crate)` — never frozen);
  promotion to `pub` is the sanctioned test seam (assemble_brief precedent).
- Paper acceptance needs a REAL filled option position (acquire via marketable
  `option-buy` if none) + gateway Read-Only API off — environmental, not impl-attributable.
