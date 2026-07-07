# journal — sma-signal

## seq=1 · 2026-07-07T07:34:10Z · prd→arch · completed · by=cc/claude-opus-4-8
done:   Wrote PRD.md for sma-signal — a NEW read-only command `omi sma-signal [SYMBOL...]` reporting the
        Faber 200-day month-end timing signal (HOLD/EXIT) per symbol + drift context. Provenance: the
        strategy-lab bake-off proved 200SMA month-end timing is the one edge over buy-hold on the liquid
        panel. Two operator decisions locked via /think: read-only signal command (no write path, no gate)
        + month-end cadence (Faber, least whipsaw). Code-first survey confirmed: history.rs historical_data
        (Day/Trades/"2 Y") is directly reusable for bars; delayed data irrelevant (historical closes);
        read-command pattern (own module, no gate, default paper port) applies; positions() enables the
        no-args ergonomic. Pure sma_signal(bars,n) is the frozen heart.
output: .pipeline/sma-signal/PRD.md · .pipeline/current.json (stage=prd, feature=sma-signal)
--- handoff ---
>>> NEXT
Run pipeline-arch on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
Model: frontier SOTA required — operator assigns the bot; the pipeline can't verify the model.
First: git pull --rebase; no .env needed (offline arch work).
Read for context (before acting):
  - oh-my-ib/CLAUDE.md + AGENTS.md — repo conventions; ADR 0017 (write containment — NOTE: sma-signal is
    READ-ONLY so it does NOT apply); read-command pattern (quote.rs/history.rs/positions.rs).
  - .pipeline/sma-signal/PRD.md — what (problem, goal, locked D-decisions, the pure signal contract,
    scope, success criteria, gotchas, and a "For arch" section listing exactly what to resolve)
  - src/ib/history.rs (the historical_data call to reuse + the ibapi bar type — b.date/b.close),
    src/ib/positions.rs (positions() for no-args symbol resolution), src/ib/quote.rs (read-command shape),
    src/cli.rs (Command enum + read-args pattern), src/main.rs (dispatch), src/ib/mod.rs (re-exports)
Your task (concrete, numbered):
  1. grill-with-docs. Do NOT re-open the 2 locked decisions (read-only; month-end cadence). Resolve:
     (a) module placement — new READ module src/ib/signal.rs (pure sma_signal + gateway sma_signal_cmd),
     confirm NO gate / NOT trade.rs. (b) THE one real unknown: the exact ibapi bar `date` accessor to
     extract (year, month) for month-end grouping (history.rs only `{:?}`-formats it today — find the
     typed accessor). (c) how the gateway maps data.bars → Bar{ym, close} + factor/inline the fetch.
     (d) pin the Bar / SmaSignal types + sma_signal signature for the freeze. (e) no-args→positions()
     resolution + JSON output shape. Author ADR 0034 (read-only 200-SMA month-end signal + strategy-lab
     provenance + Phase-2 sma-tick deferral).
  2. Write .pipeline/sma-signal/arch.md (chosen shape + component boundaries + the exact Rust type
     signatures to freeze). Land CONTEXT.md + docs/adr/0034-*.md inline.
  3. Set current.json.stage=arch. Append seq=2 handoff to journal.md. git add arch.md + CONTEXT.md +
     docs/adr/0034-*.md + current.json + journal.md; commit once; push.
Feature gotchas (project-specific traps you MUST know):
  - sma-signal is READ-ONLY: NO --live/OMI_ALLOW_LIVE gate, default paper port; ADR 0017 containment is
    irrelevant (no writes). Do not add write-path machinery.
  - Month-end = last TRADING day of the month in the bar series (group by year,month, take last).
  - Use the last COMPLETED month-end (exclude the in-progress final month) — the standing signal.
  - SMA computed AS OF the month-end (Faber), and ALSO at the latest bar (drift) — report both.
  - The ibapi bar date accessor for (year,month) is the one thing to verify against the crate.
  - Delayed data is fine (historical closes). Insufficient history (<n bars) ⇒ state=INSUFFICIENT, no crash.
Done when: arch.md + CONTEXT.md + ADR 0034 committed & pushed; Bar/SmaSignal + sma_signal signature pinned
precisely enough for pipeline-task to write red tests without re-deciding.
On success: stage→arch, then run pipeline-task (decompose into atomic card(s), write a red test).
On failure: attempts++; >=3 ⇒ blocked ⇒ run pipeline-hunt.
<<< END
