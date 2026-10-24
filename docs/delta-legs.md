# `omi spreads` + `omi delta-legs` — read-only

Both commands read. Neither places an order, neither accepts `--confirm`. To trade
what they show, use the existing write verbs (`covered-call` / `short-put` /
`option-close`), which preview by default and send only with `--confirm`.

## `omi spreads`

Income-book view of what is already held, from one `account_updates` drain.

- `covered`: every short call (right `C`, qty < 0) with the underlying's coverage —
  `shares_held` (long stock only), `short_calls` (all short calls on the symbol,
  every expiry and strike), `free_shares = shares_held − 100 × short_calls`.
- `short_puts`: every short put with `assignment_notional = strike × 100 × |qty|`.
- Top-level `assignment_notional` is the sum of the short puts' notionals.
- Long options and bare stock are not rows. Stock only feeds the coverage numbers.
- `close_hints` prints the exact close command shapes; nothing is sent.

## `omi delta-legs SYMBOL`

Picks covered-call and cash-secured-put strikes for one underlying.

```
omi delta-legs AAPL --kind both --dte-min 21 --dte-max 45 --dte-target 30 --delta-min 0.15 --delta-max 0.25
```

`--kind` is `covered` (calls), `csp` (puts), or `both` (default).

### |δ| band — inclusive

Default band is **`[0.15, 0.25]`, inclusive at both ends** (`DELTA_MIN` / `DELTA_MAX`).
`0.15` and `0.25` are in; `0.149` and `0.251` are out. |δ| is `delta.abs()`, so a put
quoted at `−0.20` qualifies. Override with `--delta-min` / `--delta-max`; both must be
finite and in `(0, 1)` with `min <= max`.

A leg outside the band is **omitted**, not listed. `out_of_band` is always `[]`.
`candidates` and `recommended` are the same array (the 推荐带).

### Greeks source

Delta and IV come from the option snapshot's model computation row only:
`ModelOption` (tick 13) or `DelayedModelOption` (tick 83), last-write-wins — the same
rule as `option-quote` (ADR 0019). Bid/ask/last greeks are per-side IVs and are
ignored.

No OPRA entitlement, or a delayed farm that never sends a model row, means the leg
has no delta. It is dropped from the recommended band. If **no** quoted leg carried a
model delta the output is `candidates: []` plus `degraded: "no_model_greeks"`, and the
command still exits 0. There is no synthetic Black-Scholes fallback.

### Annualization — approximate

`mid = (bid + ask) / 2`, and only when both are `> 0`. Otherwise the leg is not
ranked.

- Call (covered): `(mid / spot) × (365 / dte)`
- Put (CSP): `(mid / strike) × (365 / dte)` — premium against the per-share
  assignment notional

This is linear in `365 / dte`. It is not IV-adjusted, not discounted, and ignores
early assignment and dividends. `assignment_notional` (`strike × 100 × 1`) is reported
on puts so margin and assignment exposure stay visible; calls carry `null`.

### KPI

- Hard floor: annualized `>= 0.10`. A leg under it is kept and flagged
  `above_floor: false` — the band is about delta, not yield.
- Operator target: `0.15–0.20`, flagged `in_target_band`.
- Sort aim: `0.175`, the midpoint. Primary sort is distance of the annualized yield
  to `0.175` (a leg with no yield sorts last); then distance of |δ| to `0.20`; then
  strike ascending.

### Ranking is not a trade recommendation

**排序不是建议下单。** The order is a distance-to-target sort so the operator can scan.
It is not advice to sell any contract. The `disclaimer` field says this in both
languages. Sending an order is a separate command and stays behind its own preview.
