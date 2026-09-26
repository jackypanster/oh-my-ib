# PRD — pcs-scan

Feature: read-only `omi pcs-scan SYM...` — rank put-credit-spread (PCS) candidates across a watchlist by the
operator's parameters, in minutes, from IB real-time option quotes + IB model greeks.
Status: decision-complete (planned + approved 2026-09-26 in session; operator: "没有偏好 / 同意默认排除 / 同意风控 /
不要把简单事情复杂化 / 同意"). Read-only — `src/ib/trade.rs` untouched; NO order placement. The tool ranks by
operator-given params; it never picks a trade (not investment advice).

## Problem

Operator wants PCS candidates for a ~38-symbol daily watchlist. Existing tools cannot do it at useful speed:
`option-quote` is one contract per invocation (one connect + ≈11s IB snapshot tail each) ⇒ 38 symbols × ~10
strikes ≈ 1h+, and ranking by hand/LLM arithmetic is error-prone. `option-chain` already works live (≈3s,
2026-09-26 scan of all 46 watchlist names); option quotes need OPRA (operator subscribing once non-pro review lands).

## Goal

One invocation scans N symbols and emits deterministic, agent-parseable JSON: per symbol either ONE best PCS
candidate (with all numbers needed to decide) or a skip reason. Whole 38-symbol list in ≈2–3 min.

## Success criteria (acceptance)

1. `omi pcs-scan --help` documents symbols (variadic, ≥1) and flags `--dte-min 21 --dte-max 45 --dte-target 30
   --delta 0.20 --width 5 --min-credit-ratio 0.25` (defaults shown); `omi help` lists `pcs-scan` as `read-only`.
2. Output: `{"params":{…},"candidates":[…],"skipped":[{"symbol","reason"}]}` — candidates sorted by
   `return_on_risk` desc; every input symbol appears exactly once (candidate XOR skipped).
3. Candidate fields: `symbol, spot, expiry, dte, short{strike,bid,ask,mid,delta,iv}, long{strike,bid,ask,mid},
   width, credit_mid, credit_natural, max_loss, return_on_risk, breakeven, pop_approx, spread_pct`.
   Math: `credit_mid = short.mid − long.mid`; `credit_natural = short.bid − long.ask`;
   `max_loss = (width − credit_mid) × 100`; `return_on_risk = credit_mid × 100 / max_loss`;
   `breakeven = short.strike − credit_mid`; `pop_approx = 1 − |short.delta|`;
   `spread_pct` = mean of the two legs' `(ask − bid)/mid`.
4. Skip reasons (closed set, snake_case): `no_expiry_in_window`, `spot_unavailable`, `no_greeks`, `no_long_leg`,
   `below_min_credit`, `quote_error`. One bad contract/symbol never fails the command (skip-not-fail).
5. Pre-connect usage errors (exit 64): no symbols; `dte-min > dte-max`; `delta` ∉ (0,1); `width ≤ 0`;
   `min-credit-ratio` ∉ [0,1). Dead port ⇒ `code="connection"`.
6. Verify gate: `cargo build` · `cargo clippy --all-targets -- -D warnings` · `cargo test` green.
7. Operator live acceptance (after OPRA is active, US RTH, `:4001`): `omi --live --md-type live pcs-scan <38
   symbols>` completes ≤ ~5 min with ≥1 candidate and no hang; spot-check one candidate's leg prices against
   `omi option-quote`.

## Scope

- New read-only command `pcs-scan` (cli.rs args, main.rs dispatch, surface.rs help registry + `command_name`).
- New module `src/ib/pcs_scan.rs`: (a) gateway fetch; (b) PURE selection/scoring/ranking seams.
- Fetch flow on ONE connection, md-type from the global `--md-type` (agent passes `live`):
  1. spot: batch stock snapshots for all symbols (concurrent, ≤ batch cap), reuse ADR 0038 total-deadline drain.
  2. per symbol `option_chain` (reuse option-chain resolution) → pick ONE expiry: the one in
     [dte-min, dte-max] whose DTE is nearest `dte-target` (tie ⇒ earlier).
  3. puts for that expiry with strikes in [0.70×spot, spot] → batch option snapshots (concurrent, ≤ batch cap per
     batch), collecting bid/ask (`quote_price_tick`) + model greeks (`option_quote_greeks`).
  4. pure select: short = put with model delta whose |delta| is nearest `--delta` (tie ⇒ lower strike);
     long = the fetched put with strike == short.strike − width (absent ⇒ `no_long_leg`); `credit_mid <
     min_credit_ratio × width` ⇒ `below_min_credit`; else candidate.
- Help inventory frozen test (`tests/help_command.rs`) gains `"pcs-scan"` (re-freeze by pipeline-task).

## Non-scope

- Placing orders (execution stays `option-combo --preview` → operator confirm; live cap unchanged $500).
- Open interest / volume (needs streaming generic ticks), earnings calendar, IV rank, multi-expiry search,
  delta fallback when greeks are absent (⇒ skip `no_greeks`), watchlist storage/exclusions in code (the agent
  passes the list), index options (SPX), other structures (call spreads, condors).

## Deferred (future PRDs)

- OI/volume liquidity filter — requires streaming market data (generic ticks 101/100); snapshot mode forbids generic ticks.
- Earnings-date exclusion — no free earnings source over TWS API.
- Delta fallback (OTM% proxy) — cut for simplicity (operator: 不要复杂化); `no_greeks` skip surfaces the gap.

## Decisions

- D1 **Read-only ranking tool; params are operator's; no trade picking** — `✅ human-confirmed` (2026-09-26).
- D2 **Defaults** DTE 21–45 target 30, |Δ| 0.20, width $5, min credit 0.25×width — `✅ human-confirmed`
  (operator "没有偏好" ⇒ accepted my stated starting defaults; all CLI-overridable).
- D3 **Width $5 matches the live cap** (`DEFAULT_MAX_NOTIONAL = 500`, `src/ib/trade.rs:209`; `combo_live_max_risk`
  = |Δstrike|×100×qty, `trade.rs:272`) — `✅ human-confirmed` ("同意你的风控") + `📖 code-verified`.
- D4 **Exclusions live in the watchlist, not in code** — `✅ human-confirmed` (default exclusions agreed; memory
  `project_pcs-watchlist`).
- D5 **Simplicity cuts** (1 expiry/symbol, no OI, no delta fallback, snapshot batching not a new streaming
  engine) — `✅ human-confirmed` ("同意", after the effort/complexity review).
- D6 **Skip-not-fail per contract/symbol** — `✅ human-confirmed` (presented as the one rule change vs `quote`'s
  fail-fast, ADR 0013). Exception: connect failure / usage errors still fail the whole command.
- D7 **Reuse** `quote_price_tick` (`src/ib/quote.rs:139`), `option_quote_greeks` (`src/ib/option_quote.rs:40`),
  ADR 0038 total-deadline drain + `SNAPSHOT_DEADLINE` (`src/ib/mod.rs:68`), option-chain via
  `client.option_chain` (`src/ib/option_chain.rs`) — `📖 code-verified`.
- D8 **Concurrent snapshots on one sync client** (subscribe a batch, then drain each) — `⚠️ assumed`
  (IB serves snapshots server-side in parallel; ibapi request-id routing per `Subscription`). **Arch challenge
  target**: verify ibapi-3.1.0 allows N live `Subscription`s from one blocking `Client` simultaneously and pick
  the batch cap (≤ ~90 market-data lines; IB default 100 lines).
- D9 **Spot selection** — `⚠️ assumed` (low-risk): `Last` if >0, else mid of `Bid`/`Ask` if both >0, else `Close`;
  none ⇒ `spot_unavailable`. Arch may refine (delayed tick labels are `Delayed*`).
- D10 **Mid/credit basis** — `⚠️ assumed` (low-risk): rank + min-credit filter on `credit_mid`;
  `credit_natural` reported for fill realism. Leg with bid ≤ 0 or ask ≤ 0 ⇒ that put is not usable (treated as
  absent for long-leg, excluded for short selection).
- D11 **Operating mode = coordinated (herdr: Pi impl, grok review), full-auto to review, human-direct merge** —
  `✅ human-confirmed` ("同意" to "还按上次的方式").

## Freeze coverage (preflight for pipeline-task)

Hermetic pure seams (no gateway, no mocks): `pick_expiry(expirations, today, min, max, target)`;
`select_pcs(symbol, spot, expiry, dte, puts: &[PutRow], params) -> Result<Candidate, SkipReason>` (all math
of criterion 3 + skip reasons `no_greeks`/`no_long_leg`/`below_min_credit`); `pick_spot(ticks)`;
`rank(candidates)`; output shaping (every symbol exactly once); CLI usage errors (criterion 5, pre-connect);
help inventory + `read-only` gate. NOT freezable (review-by-reading + live acceptance): batching/concurrency,
drain reuse, option_chain wiring, per-contract error → skip mapping.

## Risks

- R1 model greeks absent under some entitlement states (observed 10091 flip-flop 2026-09-26) ⇒ many `no_greeks`
  skips; acceptable failure mode (visible), verified at live acceptance.
- R2 reqSecDefOptParams strikes are a union across expiries ⇒ some strikes absent for the chosen expiry ⇒
  per-contract error ⇒ skip-not-fail covers it.
- R3 ≈11s IB snapshot tail × number of batches bounds wall time; batch cap too small ⇒ slow, too large ⇒
  exceeds market-data lines (IB error 101).
