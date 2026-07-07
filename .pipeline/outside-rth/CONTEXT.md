# CONTEXT — outside-rth (domain language)

Ubiquitous terms this feature adds/sharpens. Ground for the cold task/impl/review nodes.

- **RTH (regular trading hours)** — the primary US session (09:30–16:00 ET). Today every `omi` STK order
  is RTH-only: `Order.outside_rth` is left at its `Default` (`false`) and never set.
- **outside-RTH / extended hours** — pre-market + post-market together, as ONE boolean. IB's
  `Order.outside_rth = true` admits an order to fill in RTH **and** pre **and** post. There is NO native
  pre-only or post-only flag; isolating one session would need `good_after_time`/`good_till_date` clock
  windows (explicitly out of scope). "Extended hours" in this repo = the `outside_rth == true` state.
- **`--outside-rth`** — the opt-in CLI flag on `buy`/`sell` (STK only). Absent ⇒ `false` ⇒ RTH-only
  (unchanged default). Present ⇒ request extended-hours eligibility. STK-only: US equity options are
  RTH-only, so the flag would be a silent no-op on `option-*` verbs and is NOT exposed there.
- **the new seam** — `apply_outside_rth(order: &mut Order, outside_rth: bool) -> Result<(), String>`:
  applies the flag to a freshly-built STK order, refusing only the MKT+outside-RTH corner. The ONLY new
  frozen surface (tested in `tests/outside_rth.rs`). Deliberately a POST-build mutator, not a
  `build_stk_order` parameter — see *frozen-arity*.
- **MKT+outside-RTH refuse** — a MKT order cannot fill outside RTH (IB queues it to the next RTH open),
  so `--outside-rth` without `--limit` is a hard refuse: `code="config"` (exit 5, a flag-COMBINATION
  error per `ErrorKind::Config`'s doc), offline/pre-connect. The limit is also the extended-hours
  slippage breaker (thin liquidity), so LMT-required is a safety property, not just an API constraint.
- **frozen-arity** — `build_stk_order`'s 4-arg signature `(symbol, side, quantity, limit)` is pinned by
  three frozen test files (`stk_orders_command.rs`, `order_preview_command.rs`, `write_path_semantics_doc.rs`).
  It MUST NOT change; the flag is applied by the new post-build seam instead. (Same containment discipline
  as ADR 0031 D6's "keep the fn, only rewire".)
- **preview echo** — `shape_preview` gains `"outside_rth"` INSIDE its `"order"` sub-object
  (`{type, qty, limit, outside_rth}`). The existing frozen preview test asserts only the 8 TOP-LEVEL keys
  + `order.limit`, so the nested add is non-breaking. `--preview` is the read-only verification surface:
  `omi buy … --outside-rth --preview` shows `order.outside_rth: true, transmits: false`.
- **orthogonal to risk** — `outside_rth` changes WHEN an order may fill, never HOW MUCH risk it carries.
  The double live gate, the `OMI_MAX_NOTIONAL` cap, and the combo pure-width breaker are untouched and
  stay in force. On the live path the LMT-required guard is auto-satisfied (live is already LMT-only,
  ADR 0030). No risk ADR.
- **routing-gated (deferred)** — whether Tiger's gateway/account actually ROUTES extended-hours fills is
  entitlement/session-dependent. If not, `--outside-rth` is a harmless no-op (order waits for RTH). We
  prove the flag is SENT+accepted (preview + paper `omi orders`); a real pre/post fill is deferred
  operator acceptance, NOT a merge blocker (same shape as the `[460]` combo entitlement gap).

Unchanged domain (do not touch): `build_stk_order` body/signature, `shape_order_ack` (6-key transmit
ack), `place_core`, `preview_stk_option`, the live gate, `check_live_write_posture`, `resolve_max_notional`,
`combo_live_max_risk`, `option-*` verbs, `cancel`, and all paper (`:4002`) / existing-flag behavior.
