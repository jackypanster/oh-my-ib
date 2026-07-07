# ADR 0032 — outside-rth (opt-in extended-hours flag on the STK order path)

Status: accepted · 2026-07-07 · feature: outside-rth · extends ADR 0017 (write containment), ADR 0027 (read-only preview)

## Context

`omi buy`/`omi sell` place STK orders eligible to fill only during regular trading hours (RTH):
`build_stk_order` (`trade.rs:31`) leaves `Order.outside_rth` at `Default` (`false`) and nothing sets it.
The operator wants limit orders that can work the pre-market and post-market sessions.

IB's TWS API (verified against the ibapi 3.1 crate) models extended hours as a SINGLE boolean
`Order.outside_rth` — `true` = eligible RTH + pre + post together; `false` = RTH only. There is no native
pre-only/post-only selector (that would need `good_after_time`/`good_till_date` clock windows). The field
exists (`orders/mod.rs:127`, doc = IB's official text) and is wired both ways (`proto/encoders.rs:163`,
`proto/decoders.rs:165`). MKT orders cannot fill outside RTH — IB queues them to the next RTH open — so
the flag is only meaningful with a limit price.

## Decision

Add an opt-in `--outside-rth` boolean flag to `buy`/`sell` (STK only). When set on a LMT order the placed
`Order.outside_rth = true`; absent ⇒ `false` ⇒ today's RTH-only behavior byte-for-byte unchanged. The
flag is applied by a new pure seam AFTER the order is built, and refuses the MKT+outside-RTH corner.

### D1 — one boolean, NOT a 3-way pre/regular/post selector

Map 1:1 to IB's `outside_rth`. `--outside-rth` present ⇒ RTH+pre+post; absent ⇒ RTH only. Rejected:
a `--session pre|reg|post` selector — IB has no pre-only/post-only flag, so it would require emulating
windows via `good_after_time`/`good_till_date` plus a timezone + half-day-holiday calendar; DST-fragile,
much more code, and still unable to cleanly isolate pre vs post. Not worth it for the operator's need.

### D2 — STK `buy`/`sell` only

Single-name US equity options are RTH-only, so `outside_rth` is a silent no-op on `option-buy`/`sell`/
`combo`/`close`. Exposing the flag there would mislead. The flag lives on `OrderArgs` (STK verbs) only.

### D3 — LMT-required; MKT+outside-RTH is a hard refuse (config / exit 5)

`--outside-rth` without `--limit` (a MKT order) ⇒ `code="config"`, exit 5, message names the fix
("--outside-rth requires a limit price …; pass --limit"), offline/pre-connect. `ErrorKind::Config`'s doc
is literally "Bad local config or **flag combination**" (`error.rs:16`) ⇒ the correct bucket (not `usage`,
which is per-value parse failure; this is a cross-flag constraint, like the live gate). Rationale: MKT
can't fill outside RTH anyway, and the limit is the extended-hours slippage breaker (thin liquidity).
`outside_rth == false` is always `Ok` (including MKT) ⇒ flagless behavior is byte-identical.

### D4 — NEW post-build seam `apply_outside_rth`, NOT a `build_stk_order` parameter

```rust
// Ok(()) sets order.outside_rth; Err only for the MKT+true corner. Reads order_type (set by build_stk_order).
pub fn apply_outside_rth(order: &mut Order, outside_rth: bool) -> Result<(), String>
```

`build_stk_order`'s 4-arg signature is frozen by three test files that call it verbatim
(`stk_orders_command.rs:50,62`, `order_preview_command.rs:45,76,84`, `write_path_semantics_doc.rs:77,78`).
Adding a 5th parameter breaks their compilation = editing another card's `spec-paths` — forbidden
(AGENTS.md hard invariant; same discipline as ADR 0031 D6). So the flag is applied after the build. `place()`
binds `mut order`, calls `apply_outside_rth(&mut order, args.outside_rth).map_err(config)?` before
`place_core`. The seam reads `order.order_type` (`"LMT"`/`"MKT"`) for the D3 guard.

### D5 — guardrails untouched; `outside_rth` is orthogonal to risk

It changes WHEN an order may fill, not risk size. The double live gate (`require_live_write_gate`), the
`OMI_MAX_NOTIONAL` cap (`check_live_write_posture`/`resolve_max_notional`), and the combo pure-width
breaker (`combo_live_max_risk`, ADR 0031) are all unchanged. On the live path the D3 LMT-required guard is
auto-satisfied (live is already LMT-only, ADR 0030). No risk-model change, no new env, no new gate.

### D6 — preview echoes the flag, non-breakingly

`shape_preview` gains `"outside_rth": order.outside_rth` INSIDE its `"order"` sub-object. The frozen
preview test (`order_preview_command.rs:52`) asserts only the 8 TOP-LEVEL keys + `order.limit`, so the
nested add leaves it green. `preview_stk_option` forwards the same `order` built in `place()`, so once
`apply_outside_rth` runs before `place_core` both the `--preview` branch and the transmit branch carry the
flag. The 6-key transmit ack (`shape_order_ack`) is NOT extended — preview is the verification surface;
a real placed order's flag is observable via `omi orders`.

## Consequences

- `omi buy/sell … --limit … --outside-rth` places a LMT order eligible for pre/post fills; absent, RTH-only
  and byte-identical to today. `--outside-rth` on a MKT order refuses exit 5 before connect.
- New code is one clap flag + one pure seam + a one-key preview echo + a one-line `place()` wire.
  `build_stk_order`, `shape_order_ack`, `place_core`, the gate, the cap, the combo breaker, all option
  verbs, and every existing frozen test are unchanged.
- Extended-hours ROUTING is entitlement/session-gated at the broker. If unavailable, the flag is a harmless
  no-op (order waits for RTH). Verified here: the flag is SENT+accepted (preview + paper `omi orders`); a
  real pre/post fill is deferred operator acceptance, not a merge blocker.

## Freeze coverage

- **FROZEN** (`tests/outside_rth.rs`): the pure `apply_outside_rth` — LMT+true ⇒ Ok ∧ `outside_rth==true`;
  LMT+false ⇒ Ok ∧ false; MKT+true ⇒ Err(msg contains "limit"); MKT+false ⇒ Ok ∧ false. Preview echo —
  `shape_preview` on a `apply(true)` order ⇒ `order.outside_rth == true`, default ⇒ false. CLI black-box —
  `buy … --outside-rth` no `--limit` ⇒ config/exit 5; `buy … --limit 1 --outside-rth --port 65000` ⇒
  connection.
- **REVIEW-BY-READING** (not frozen): the `place()` wire (call-site, `map_err(config)`, runs before
  `place_core`); the `shape_preview` source edit (effect frozen via the echo assertion);
  `build_stk_order`/`shape_order_ack` untouched; the four prior frozen suites byte-identical + green.
- **OPERATOR ACCEPTANCE**: paper (`:4002`) `omi buy AAPL 1 --limit 150 --outside-rth --preview` shows the
  flag; a real place shows it in `omi orders`. Real post-market fill on `:4001` = deferred (entitlement).

## Alternatives rejected

- **3-way `--session pre|reg|post`.** IB has no pre/post-only flag; needs GTD windowing + timezone/holiday
  calendar — DST-fragile, heavy, still can't isolate pre vs post cleanly (D1).
- **Add `outside_rth` param to `build_stk_order`.** Breaks three frozen test files' compilation = editing
  another card's spec (D4).
- **Extend the transmit ack (`shape_order_ack`) with `outside_rth`.** More frozen-surface churn for little
  gain; preview + `omi orders` already surface it (D6).
- **Thread the flag through the option verbs for surface consistency.** No-op there, misleading (D2).
