# journal — quote-drop-volume (review-05 follow-up C)

## seq=1 · 2026-06-29T15:13:15Z · prd→arch · completed · by=claude-opus-4-8(claude-code)
done:   New feature for follow-up C. Code-first root cause: quote passes every TickTypes::Size.size
        straight through; the gateway's delayed-volume tick (1.4e13) has no reliable/constant scaling
        and is likely Tiger-gateway-specific. Operator decision (grilled): DROP volume — quote emits
        price ticks only; volume stays in history. Decision-complete PRD written; current.json repointed.
output: .pipeline/quote-drop-volume/PRD.md, .pipeline/current.json
--- handoff ---
>>> NEXT
Run pipeline-arch then task then impl (rebuild from repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
Read: .pipeline/quote-drop-volume/PRD.md; src/ib/quote.rs; src/ib/mod.rs.
Design:
  - src/ib/quote.rs: pub fn quote_price_tick(tick: &TickTypes) -> Option<(String, f64)> — Some only for
    TickTypes::Price ((format!("{:?}", p.tick_type), p.price)), None otherwise. Rewrite the loop: break on
    SnapshotEnd; insert only quote_price_tick(&tick) results; REMOVE the TickTypes::Size arm.
  - src/ib/mod.rs: pub use quote::quote_price_tick; (so the frozen test can import oh_my_ib::ib::quote_price_tick).
Freeze: tests/quote_ticks.rs (offline) — add ibapi to [dev-dependencies]; construct a TickTypes::Price
  (=> Some) and a TickTypes::Size volume tick (=> None). NEW spec-rev. Confirm TickPrice/TickSize fields
  are pub + constructible (ibapi market_data::realtime) before writing the test.
Live acceptance: omi --live quote AAPL --md-type delayed => price keys, NO *Volume/size key, valid JSON.
Gotcha: do NOT touch the other frozen specs (13e522d phase1, a072015 tz, fd72d90 connect-retry).
Done when: arch.md + freeze/record + impl + PR; merge human-confirmed.
On failure: attempts++; >=3 ⇒ blocked ⇒ pipeline-hunt.
<<< END
