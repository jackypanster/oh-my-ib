# arch — outside-rth

Stage: arch · feature: outside-rth · repo: jackypanster/oh-my-ib · branch: main
Extends ADR 0017 (write containment) + ADR 0027 (read-only preview). Decision record:
`docs/adr/0032-outside-rth.md`. Domain: `CONTEXT.md`.

## Chosen shape

One CLI flag + one NEW pure seam + a one-key preview echo + a call-site wire in `place()`. The seam
decides "may this order carry outside-RTH, and if so set the flag" — a MKT+outside-RTH combination is
refused, everything else sets `order.outside_rth`. No new struct, no new command, no new env, no state,
no gateway round-trip, no risk-path change. The refuse is offline-deterministic ⇒ freezable.

**Why a new seam and not a `build_stk_order` parameter** (the load-bearing arch call): `build_stk_order`'s
4-arg signature is frozen by THREE test files that call it verbatim —
`tests/stk_orders_command.rs:50,62`, `tests/order_preview_command.rs:45,76,84`,
`tests/write_path_semantics_doc.rs:77,78`. Adding a 5th parameter breaks their compilation = editing
another card's `spec-paths` (AGENTS.md hard invariant, same trap as ADR 0031 D6). So the flag is applied
AFTER the build, by a seam that reads the already-set `order.order_type` for the MKT guard.

```
place (trade.rs:603)                              [gateway fn — review-by-reading]
  qty > 0            validation                   unchanged  (:605, usage/exit 64)
  limit > 0 if Some  validation                   unchanged  (:611, usage/exit 64)
  let (contract, mut order) = build_stk_order(sym, side, qty, limit)   UNCHANGED 4-arg (:621)
  apply_outside_rth(&mut order, args.outside_rth)? ── NEW pure seam ── Ok(()) | Err(mkt+ortH)
                                                    every Err → AppError::config (exit 5)
  place_core(cfg, ctx, &contract, &order, ack)    unchanged (:623)
     └─ if cfg.preview → preview_stk_option(&order) → shape_preview(&order)   sees the flag
     └─ else           → gate → connect → allocate → place                    sees the flag
```

`apply_outside_rth` runs BEFORE `place_core`, so the ONE `order` reaches BOTH the preview branch
(`preview_stk_option` `:515` forwards it) and the transmit branch. Ordering preserved:
usage (qty/limit) < config (this guard, + the downstream gate) < connection.

## The new pure seam

```rust
/// Apply the opt-in outside-RTH flag to a freshly-built STK order. A MKT order (order_type "MKT")
/// cannot fill outside regular trading hours, so outside_rth on a MKT order is refused; otherwise the
/// flag is set verbatim. Pure/offline ⇒ frozen. Reads order_type (set by build_stk_order), sets outside_rth.
pub fn apply_outside_rth(order: &mut Order, outside_rth: bool) -> Result<(), String> {
    if outside_rth && order.order_type == "MKT" {
        return Err("--outside-rth requires a limit price (MKT orders do not fill outside regular \
                    trading hours); pass --limit".into());
    }
    order.outside_rth = outside_rth;
    Ok(())
}
```

- `outside_rth == false` is ALWAYS `Ok` (sets `false`, i.e. today's default) — even for MKT. Only the
  `true && MKT` corner refuses. This keeps every existing (flagless) call path byte-identical in behavior.
- `order.order_type` is `"LMT"` or `"MKT"` (a `String`, set at `build_stk_order` `:41`/`:45`).

## Component boundaries / write-set (for the impl card)

- `src/cli.rs` (`OrderArgs` `:106`) — ADD `#[arg(long)] pub outside_rth: bool`. clap defaults `bool`
  to `false` (flag absent ⇒ false). Shared by `Buy`/`Sell` (both wrap `OrderArgs`). No `short`, no
  `conflicts_with` (the MKT conflict is a runtime refuse with a helpful message, not a clap-level block —
  clap can't see the limit/MKT relationship cleanly, and the message must name the fix).
- `src/ib/trade.rs` — ADD `pub fn apply_outside_rth(...)` (above). WIRE into `place()`: after
  `build_stk_order` (bind `mut order`), call `apply_outside_rth(&mut order, args.outside_rth)
  .map_err(|m| AppError::config(m, ctx))?` before `place_core`. ADD `"outside_rth": order.outside_rth`
  INSIDE the `"order"` object of `shape_preview` (`:92-96`).
- `src/ib/mod.rs:45` — add `apply_outside_rth` to the `pub use trade::{…}` list.
- `tests/outside_rth.rs` — NEW frozen spec (written by pipeline-task, not impl).

Out of write-set (MUST stay unchanged, byte-identical): `build_stk_order` (its 4-arg signature + body),
`shape_order_ack` (stays 6-key — preview is the verification surface, not the transmit ack), `place_core`,
`preview_stk_option`, the live gate / `check_live_write_posture` / `resolve_max_notional` /
`combo_live_max_risk`, all option/combo/close verbs, and the frozen tests
`tests/{stk_orders_command,order_preview_command,write_path_semantics_doc,live_write_guardrail}.rs`.

## Freeze plan (for pipeline-task)

FREEZE in NEW `tests/outside_rth.rs` (import `oh_my_ib::ib::{build_stk_order, apply_outside_rth,
shape_preview}`):

- **seam (offline):** build a LMT order (`build_stk_order("AAPL", Buy, 1.0, Some(150.0))`) →
  `apply_outside_rth(&mut o, true)` ⇒ `Ok(())` ∧ `o.outside_rth == true`; a fresh LMT order +
  `apply(false)` ⇒ `Ok` ∧ `o.outside_rth == false`. Build a MKT order (`.., None`) + `apply(true)` ⇒
  `Err(_)` (message contains "limit"); MKT + `apply(false)` ⇒ `Ok` ∧ `o.outside_rth == false`.
- **preview echo (offline):** LMT order + `apply(true)` → `shape_preview(json!({}), &o, 1.0, "USD")` ⇒
  `out["order"]["outside_rth"] == json!(true)`; a default order ⇒ `== json!(false)`. (The top-level key
  set is asserted UNCHANGED by the existing `order_preview_command.rs` — this feature's test need only
  assert the nested field.)
- **CLI guard (black-box, mirrors `order_preview_command.rs` style):** `omi --format json buy AAPL 1
  --outside-rth` (no `--limit`) ⇒ `code="config"` (exit 5), no gateway needed; `omi --format json buy
  AAPL 1 --limit 1 --outside-rth --host 127.0.0.1 --port 65000` ⇒ `code="connection"` (guard passed).

NOT frozen (review-by-reading): the `place()` wiring (`apply_outside_rth` call-site + `map_err` bucket +
that it runs before `place_core`), and the `shape_preview` source edit (its effect IS frozen via the echo
assertion). Single spec-rev for this feature.

## Reference-behavior artifact

NOT APPLICABLE. No new external-reference dependency. `Order.outside_rth: bool` is a documented ibapi 3.1
field, wired over the wire (encoders/decoders verified in the PRD); setting a bool needs no wire probe.
The gateway's actual extended-hours ROUTING is entitlement/session-dependent (deferred acceptance,
criterion 7) — a known-unknown recorded as such, not a design risk to freeze.

## Verify (unchanged 4 gates)

`cargo build` · `cargo test` (full; the four named frozen suites still 100% green — proof `build_stk_order`
and `shape_preview`'s top-level shape are untouched) · `cargo clippy --all-targets -- -D warnings`
(`apply_outside_rth` is `pub` + re-exported + referenced by the frozen test ⇒ no `dead_code`). Offline
checks need NO gateway. Live-pass = operator paper (criterion 6) now; real pre/post fill deferred.
