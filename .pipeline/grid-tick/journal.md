# journal — grid-tick

## seq=1 · 2026-07-07T03:19:24Z · prd→arch · completed · by=cc/claude-opus-4-8
done:   Wrote PRD.md for grid-tick — a new `omi grid-tick` subcommand (single-tick, cron-scheduled,
        single-connection, stateless reconcile-to-desired-state). Design fully grilled & locked via
        /think across this session: 5 operator decisions (D-CMD Rust subcommand not script/daemon;
        D-CASH floor = 50% of net_liquidation; D-FLAT idle+cancel when qty==0; D-TARGET paper-only v1;
        D-MAXSH per-symbol ceiling default 300) + the pure planner contract (plan_grid_tick, the frozen
        heart). Code-first survey confirmed: notional cap paper-exempt; one account_updates drain feeds
        cash+positions; open_orders_with_client precedent makes single-connection mechanical; toml+serde
        stack already present.
output: .pipeline/grid-tick/PRD.md · .pipeline/current.json (stage=prd, feature=grid-tick)
--- handoff ---
>>> NEXT
Run pipeline-arch on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
Model: frontier SOTA required — operator assigns the bot; the pipeline can't verify the model.
First: git pull --rebase; load repo config (~/.config/oh-my-ib/config.toml is user-local, NOT in repo;
no .env needed for offline arch work).
Read for context (before acting):
  - oh-my-ib/CLAUDE.md + AGENTS.md — repo conventions; ADR 0017 write-containment (writes ONLY in
    trade.rs), ADR 0030/0031 gates (double live gate, $500 notional cap). Read FIRST.
  - .pipeline/grid-tick/PRD.md — what (problem, goal, the locked D-decisions, the pure planner contract,
    scope, success criteria, gotchas, and a "For arch" section listing exactly what you must resolve)
  - src/ib/trade.rs (place_core :582, place :617, cancel :429, check_live_write_posture, build_stk_order),
    src/ib/orders.rs (open_orders_with_client — the &Client inner pattern to mirror), src/ib/account.rs
    (SummaryAccumulator), src/ib/positions.rs (position_row, avg_cost), src/config.rs (toml load pattern)
Your task (concrete, numbered):
  1. grill-with-docs the design against the codebase. Do NOT re-open the locked D-decisions — resolve the
     5 items in PRD.md "For arch": (a) module placement + write-containment — new pure planner+config
     module vs trade.rs; author ADR 0033 (strategy/policy layer + reconcile model + containment extension:
     grid orchestrates, gated place_core/cancel stay authoritative). (b) confirm place_core + cancel split
     cleanly into `_with_client` &Client inners mirroring open_orders_with_client, and that ONE
     account_updates drain can feed both SummaryAccumulator and Vec<position_row>. (c) config shape +
     location (dedicated --config file vs [grid] table in the existing config.toml) + serde defaults +
     validation → code="config". (d) Action→execution mapping on the shared client + the command's JSON
     output shape (actions / --dry-run plan / no-op). (e) CONTEXT.md glossary; confirm NO existing risk
     seam changes.
  2. Write .pipeline/grid-tick/arch.md (chosen shape + component boundaries + the plan_grid_tick type
     signature the task stage will freeze). Land CONTEXT.md additions + docs/adr/0033-*.md inline
     (grill-with-docs sanctioned inline write).
  3. Set current.json.stage=arch. Append your seq=2 handoff to journal.md. git add arch.md + CONTEXT.md +
     docs/adr/0033-*.md + current.json + journal.md; commit once; push.
Feature gotchas (project-specific traps you MUST know):
  - Write-containment (ADR 0017): order-placement code lives ONLY in trade.rs today. The grid driver
    CAUSES writes — this is THE arch decision (ADR 0033). Keep actual place/cancel going through the
    gated place_core (gate+stamp+notional); the grid layer decides WHAT, trade.rs still does the write.
  - The pure planner has NO client → 100% offline-freezable; it is where ~all tests live. Keep the driver
    thin (I/O only). Freeze surface = plan_grid_tick + config parse; driver = review-by-reading + paper.
  - Notional cap is PAPER-EXEMPT (trade.rs:596) → 100-share orders work on :4002; v1 is paper-only.
  - Grid owns ALL orders on a configured symbol (reconcile cancels non-matching ones); never touches
    unconfigured symbols. This blast-radius rule is load-bearing — bake it into the planner contract.
  - Cash floor is SOFT/per-tick (net_liquidation basis); max_shares uses the strict `qty+lot<=max` form
    (never exceed). Both are grid-local, NOT the $500 cap.
Done when: arch.md + CONTEXT.md + ADR 0033 committed & pushed; the plan_grid_tick signature + Action enum
+ config struct are pinned precisely enough for pipeline-task to write red tests without re-deciding.
On success: stage→arch, then run pipeline-task (decompose into atomic cards, write a red test per card).
On failure: attempts++; >=3 ⇒ blocked ⇒ run pipeline-hunt.
<<< END
