# CONTEXT — options-read

New domain terms. Base glossary: `.pipeline/phase1-readonly/CONTEXT.md`; reuses **bounded
wait / Instant-classified None** (ADR 0016), **take-first timeout const** (ADR 0012),
**write path / double gate** (stk-orders CONTEXT — untouched here). Deltas:

## Options domain

- **Option chain** — the discovery set for one underlying: which option contracts exist.
  Delivered by reqSecDefOptParams as one row per (exchange, trading class): multiplier +
  the full expirations × strikes grid. Discovery only — no prices.
- **Underlying / conid** — the stock the option derives from; reqSecDefOptParams requires
  its IB contract id (conid), resolved via `contract_details` (FIRST row rule, ADR 0019).
- **Trading class** — the option family name under one underlying; usually == symbol
  (AAPL), but index options split families (SPX = monthly AM-settled, SPXW = weekly
  PM-settled). `--trading-class` disambiguates a quote when a symbol carries several.
- **Right** — Call or Put. CLI accepts C|CALL|P|PUT case-insensitively; output echoes
  normalized `"C"`/`"P"`.
- **Expiry** — 8-digit YYYYMMDD string (IB wire format). Lexicographic sort ==
  chronological sort, which is why chain expirations sort as plain strings.
- **Multiplier** — contract size (shares per contract), a STRING on the IB wire; US equity
  options default `"100"` (the OptionBuilder default).
- **Greeks** — the risk sensitivities (implied_volatility, delta, gamma, vega, theta) +
  option_price + underlying_price, computed by IB and delivered as OptionComputation ticks.
- **Option computation row** — one OptionComputation tick; its `field` names the price it
  was computed FROM: bid (10) / ask (11) / last (12) / **model (13)** / custom (53), plus
  delayed twins (80–83). Only the **model** rows (13, 83) are "the greeks" (what TWS shows).
- **Best-effort greeks** (ADR 0019) — model rows may simply never arrive under
  delayed+snapshot; the `greeks` key is then absent and that is a VALID success output,
  never an error. Presence of `greeks` == a model row arrived before SnapshotEnd.
- **Chain drain** — the End-bounded stream class of reqSecDefOptParams
  (SecurityDefinitionOptionParameterEnd ⇒ clean iterator end), timeout-wrapped like
  completed-orders (the wedge posture). Distinct from the **snapshot drain**
  (SnapshotEnd-bounded, bare) that option-quote shares with quote.

## Conventions (feature-specific)

- READ-ONLY feature: no write calls, `trade.rs` untouched, normal review polarity.
- `quote.rs` is frozen byte-identity (ADR 0013): reuse its pub seams (`quote_price_tick`)
  via re-exports; never edit the file.
- Chain output determinism: expirations + strikes ascending, rows by (exchange,
  trading_class) — gateway sets are unordered.
- Validation precedes connection (usage envelopes are offline-frozen): right / strike > 0 /
  expiry shape.
- Acceptance is paper (`:4002`); Tiger (`:4001`) reqSecDefOptParams support is a journaled
  observation, never a merge blocker.
