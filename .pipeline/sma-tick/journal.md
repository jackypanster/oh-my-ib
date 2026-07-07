# journal — sma-tick

## seq=1 · 2026-07-07T12:00:44Z · prd→arch · completed · by=cc/claude-opus-4-8
done:   Wrote PRD.md for sma-tick — the active counterpart to sma-signal: each month reconcile the QQQ
        position to the 200-day month-end signal (HOLD ⇒ target lot=10 shares, EXIT ⇒ 0). WRITE feature
        (real orders), grid-tick shape. Four decisions locked via /think: binary target (not ladder);
        QQQ-only + lot=10 + flags (no toml); paper-only v1 (10 QQQ ≈ $7.2k ≫ $500 cap); pure
        plan_sma_tick frozen. Code-first survey confirmed all reuse: sma_signal (PR #31), place_with_client
        + build_stk_order (grid-tick, pub(crate)), positions(), LIVE_PORT. Almost no new write code.
output: .pipeline/sma-tick/PRD.md · .pipeline/current.json (stage=prd, feature=sma-tick)
--- handoff ---
>>> NEXT
Run pipeline-arch on a FRESH session (rebuild from repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
Model: frontier SOTA required.
First: git pull --rebase.
Read for context:
  - oh-my-ib/CLAUDE.md + AGENTS.md — ADR 0017 write-containment; grid-tick as the write-reconcile precedent.
  - .pipeline/sma-tick/PRD.md — locked decisions, the pure plan_sma_tick contract, scope, gotchas, For-arch.
  - .pipeline/grid-tick/docs/adr/0033-grid-tick.md — the write-orchestration containment pattern to mirror.
  - .pipeline/sma-signal/docs/adr/0034-sma-signal.md — the signal it reuses.
  - src/ib/trade.rs (place_with_client :470 pub(crate), build_stk_order), src/ib/signal.rs (sma_signal +
    the bar fetch to share/re-call), src/ib/positions.rs (positions()), src/config.rs (LIVE_PORT),
    src/ib/grid.rs (the grid-tick driver pattern to mirror), src/cli.rs, src/main.rs, src/ib/mod.rs.
Your task (numbered):
  1. grill-with-docs. Do NOT re-open the 4 locked decisions. Resolve: (a) module = new src/ib/sma_tick.rs
     (pure plan_sma_tick + gateway sma_tick_cmd); confirm paper-only guard + containment (no raw
     place_order — compose build_stk_order + place_with_client). (b) THE order-type decision: MKT DAY
     (accept [399] queued-to-open as SUBMITTED, not error — check how omi buy currently treats [399]) vs
     marketable LMT at latest close. (c) how the gateway computes the signal (share a bar-fetch helper
     with sma_signal_cmd, or re-call historical_data) + reads current qty from positions(). (d) pin
     plan_sma_tick + TickAction for the freeze. (e) JSON shape + --dry-run. Author ADR 0035.
  2. Write arch.md + CONTEXT.md + docs/adr/0035-*.md. Set current.json.stage=arch. Append seq=2 handoff.
     Commit once; push.
Feature gotchas:
  - WRITE feature: paper-only (refuse LIVE_PORT before connect); compose trade.rs choke points (ADR 0017).
  - Binary target (lot or 0), never accumulate. plan_sma_tick pure/frozen; gateway review-by-reading.
  - [399] MKT-queued-to-open should be a SUCCESS for a monthly timing order (matches Faber act-at-open).
  - Reuse sma_signal (pure) + place_with_client; don't reimplement.
Done when: arch.md + CONTEXT.md + ADR 0035 pushed; plan_sma_tick/TickAction pinned for task.
On success: stage→arch, run pipeline-task. On failure: attempts++; >=3 ⇒ blocked ⇒ pipeline-hunt.
<<< END
