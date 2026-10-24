# arch — reprice

Stage: arch · feature: reprice · repo: jackypanster/oh-my-ib
Decision record: `docs/adr/0040-reprice.md`. Domain: `CONTEXT.md`.

## Chosen shape

One new subcommand. The decision of "preview vs submit vs block" is a pure function of
an open-order snapshot, so the four required cases are offline tests with no gateway.
The gateway function only: connect → find the open order → `plan_reprice` → (submit arm)
`place_order(existing_id, &contract, &reprice_order(...))`.

```
reprice (trade.rs)
  order_id >= 1, price finite > 0     usage/64     before connect
  if --confirm: require_live_write_gate            config/5, before connect (frozen)
  resolve_max_notional                guard/64     before connect, both arms
  connect → all_open_orders → match id
  missing order                       data/4       nothing sent
  plan_reprice(confirm, is_live, cap, target, price)
     bad price                        usage/64
     not LMT or not STK/OPT           data/4
     !confirm                         Preview JSON, transmits=false, NO place_order
     confirm && live notional > cap   guard/64, NO place_order
     confirm && ok                    Submit { order_id: THE SAME id, new_price }
  submit arm:
     reprice_order: clone, set order_id, LMT, limit, transmit=true. Nothing else.
     account empty ⇒ stamp resolved account; else keep the working order's account
     place_order(plan.order_id, ...)  — next_order_id() is NOT called
     first ack → exit 0 JSON (modified: true)
     rejection notice → rejected/7
     timeout / transport → unconfirmed/8, Do NOT retry
```

`first_place_status` is the shared ack loop. `place_with_client` (buy/sell/options/combo/close)
and `cancel_with_client` call `classify_submit_error` / `submit_timeout_error` so existing
writes use 4/7/8 too. Their success ack JSON is unchanged.

## Seams (frozen by tests/reprice.rs)

- `plan_reprice(confirm, is_live, cap, &RepriceTarget, new_price) -> Result<RepricePlan, AppError>`
- `reprice_order(&Order, order_id, new_price) -> Order` — same id, limit replaced, transmit true
- `submit_timeout_error(order_id, ctx) -> AppError` — code `unconfirmed`, exit 8, "Do NOT retry"
- `classify_submit_error(&ibapi::Error, order_id, ctx, sent) -> AppError`
  - rejection notice → 7, whether or not `sent`
  - `sent || transport` → 8
  - else (local, not sent) → 4
- `shape_reprice_preview` keys: `preview`, `transmits`, `order_id`, `current_order`,
  `new_price`, `notional`, `notional_currency`, `risk_blocked`

## Write-set

- `src/cli.rs` — `Command::Reprice(RepriceArgs)`, `--price`, positional `order_id`, `--confirm`
- `src/main.rs` — dispatch
- `src/surface.rs` — registry + `command_name` (exhaustive)
- `src/error.rs` — `Rejected`/`Unconfirmed`/`Guard` (additive; existing kinds keep their exits)
- `src/ib/trade.rs` — seams + `reprice` + write-path error mapping
- `src/ib/mod.rs` — re-exports
- `AGENTS.md`, `CLAUDE.md` (pointer only), `docs/write-path-semantics.md` (modify section + paper probe)
- `tests/reprice.rs` — new
- `tests/help_command.rs` — inventory re-freeze, add `"reprice"` only

Out of write-set: `build_stk_order` arity, `shape_order_ack` keys, `shape_preview` keys,
`require_live_write_gate` (still config/5), read-timeout exit 6, option/combo success JSON.

## Tiger

⚠️ UNVERIFIED. The probe recipe is in `docs/write-path-semantics.md` §Modify. A rejection or a
second order on paper is a failed acceptance, not a cue to cancel+replace inside this feature.
