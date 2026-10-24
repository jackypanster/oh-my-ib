# CONTEXT — reprice

- **reprice** — `omi reprice --price P ORDER_ID [--confirm]`. Changes the limit of one open
  single-leg LMT (STK or OPT). Default is a preview. `--confirm` transmits.
- **same-id modify** — TWS/ibapi modify = `placeOrder` with the existing order id. Not a new
  order. Not cancel+replace. `Order.order_id` and the `place_order` argument are that same id.
- **preview** — JSON with `transmits: false`. No `place_order`. Skips the live-write env gate.
- **confirm** — the transmit flag. Runs `require_live_write_gate` before connect.
- **notional** — `quantity × |new_price| × multiplier`. OPT multiplier 100 unless the contract
  string parses to a positive finite number. Re-checked against `OMI_MAX_NOTIONAL` on confirm
  when `cfg.port == 4001`. Paper is exempt. Boundary `notional == cap` is allowed (`>` refuses).
- **guard / 64** — risk-cap, invalid `OMI_MAX_NOTIONAL`, live MKT, live combo defined-risk
  refuse. Order not sent. Distinct JSON `code` from `usage`, same exit.
- **unconfirmed / 8** — write was sent, no definitive ack. Never auto-retry. Read timeout
  stays 6.
- **rejected / 7** — broker notice in 200–399 except 202 (cancel confirmation).
- **data / 4** — precondition, order not sent (missing, not LMT, not STK/OPT, local validation).
- **Tiger UNVERIFIED** — implement the IB path; paper acceptance must confirm. No silent fallback.
