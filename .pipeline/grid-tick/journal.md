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

## seq=2 · 2026-07-07T03:26:32Z · arch→task · completed · by=cc/claude-opus-4-8
done:   grill-with-docs resolved all 5 "For arch" items against real code. Key finding:
        place_with_client(&Client,…) ALREADY EXISTS (trade.rs:463, the ADR 0024 account-stamp choke point,
        already called by option_combo) → grid composes it + build_stk_order + a new cancel_with_client;
        NO raw ibapi write in grid ⇒ ADR 0017 grep-containment holds verbatim. Module boundary: pure
        planner+config in src/grid.rs (frozen), gateway driver in src/ib/grid.rs (review-by-reading).
        grid_tick hard-refuses live (paper-only v1) ⇒ live gate/notional machinery untouched. Authored
        ADR 0033, arch.md (with the exact Rust type signatures to freeze), CONTEXT.md glossary.
output: .pipeline/grid-tick/arch.md · .pipeline/grid-tick/CONTEXT.md ·
        .pipeline/grid-tick/docs/adr/0033-grid-tick.md · .pipeline/current.json (stage=arch)
--- handoff ---
>>> NEXT
Run pipeline-task on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
Model: frontier SOTA required — operator assigns the bot; the pipeline can't verify the model.
First: git pull --rebase; no .env needed (offline task work).
Read for context (before acting):
  - oh-my-ib/CLAUDE.md + AGENTS.md — repo conventions; agent-first authoring; ADR 0017 write-containment.
  - .pipeline/grid-tick/PRD.md — what (problem, goal, locked D-decisions, success criteria, gotchas)
  - .pipeline/grid-tick/arch.md — how: module boundaries + the EXACT Rust type signatures to freeze +
    the "For task" section (freeze coverage, card split, the clippy-on-stub trap)
  - .pipeline/grid-tick/docs/adr/0033-grid-tick.md — binding decisions (D-CMD/CONTAINMENT/TARGET/PLANNER/
    CASH/MAXSH) + Freeze coverage
  - .pipeline/grid-tick/CONTEXT.md — domain glossary
  - src/ib/trade.rs (place_with_client :463, cancel :429, build_stk_order), src/ib/orders.rs
    (open_orders_with_client), src/ib/account.rs (SummaryAccumulator), src/ib/positions.rs (position_row)
Your task (concrete, numbered):
  1. Write the RED spec tests/grid_tick.rs (spec-paths), importing from
     oh_my_ib::grid::{plan_grid_tick, GridConfig, Action, Side, AccountSnap, PositionLite, OpenOrderLite}.
     Cover ADR 0033 "Freeze coverage": (a) held symbol qty>0, cash≥floor, qty+lot≤max ⇒ exactly one
     Buy@round2(avg*(1-drop%)) qty=lot + one Sell@round2(avg*(1+rise%)) qty=min(lot,qty), no existing;
     (b) total_cash<0.5*net_liq ⇒ no Buy, Sell present; (c) qty+lot>max_shares (qty=300,lot=100,max=300 ⇒
     no Buy; qty=200,max=250 ⇒ no Buy; qty=200,max=300 ⇒ Buy) ; (d) qty==0 with a lingering order ⇒
     Cancel(order), no Place; qty==0 no orders ⇒ empty; (e) existing already matching (side+qty+|Δlimit|
     ≤0.005) ⇒ empty plan; drift ⇒ Cancel(old) then Place(new), Cancels first; (f) two symbols with
     different drop/rise% each get own pair; an order on an UNCONFIGURED symbol ⇒ no Action; (g) GridConfig
     parse — a valid toml str ⇒ defaults applied (lot=100,cash_floor_pct=50,drop=rise=2.0,max_shares=300);
     a malformed / negative-% toml ⇒ AppError code="config". Recommend testing parse via a
     GridConfig::from_toml_str(&str) seam (no filesystem) that GridConfig::load wraps.
  2. Freeze in ONE commit touching ONLY tests/grid_tick.rs — it must COMPILE-FAIL on the unresolved
     oh_my_ib::grid imports (that IS the RED; do NOT add any src/ stub — that's impl-path). This is the
     recurring clippy-on-stub trap: verify RED via the unresolved-import compile error, not a runtime fail.
     That commit's sha = the feature's single spec-rev.
  3. Record commit (metadata only): write tasks/01.md (+ tasks/02.md if you split driver from planner —
     arch.md "For task" §2 recommends: 01=pure planner+config frozen heart, 02=driver+wiring
     review-by-reading; both share the ONE spec-rev). Frontmatter: status=todo, attempts=0, verify=
     [cargo build, cargo test --test grid_tick], spec-paths=[tests/grid_tick.rs], impl-paths per arch.md
     write-set (src/grid.rs, src/ib/grid.rs, src/ib/trade.rs, src/ib/account.rs, src/cli.rs, src/main.rs,
     src/lib.rs, src/ib/mod.rs), spec-rev=<step-2 sha>. Advance current.json.stage=task. Append seq=3
     handoff to journal.md. Commit metadata (never spec-paths). Push both commits.
Feature gotchas (project-specific traps you MUST know):
  - Two-commit freeze protocol: freeze commit = ONLY tests/grid_tick.rs (spec-rev); record commit =
    cards + current.json (metadata, never spec-paths). See CONTRACT §Test ownership.
  - The planner is PURE (no client) ⇒ freeze it fully offline. The driver (src/ib/grid.rs) is
    review-by-reading — do NOT try to freeze gateway behavior; note it in the card's Freeze coverage.
  - Quantities are f64 (Order.total_quantity / p.position). Ceiling is qty+lot<=max_shares (strict).
  - Do NOT change build_stk_order's 4-arg signature (frozen by 3 other suites) — grid REUSES it as-is.
  - spec-paths ∩ impl-paths = ∅ (tests/grid_tick.rs is new, distinct from all impl files).
Done when: freeze commit (RED, compile-fail on imports) + record commit (cards + stage=task) pushed;
spec-rev recorded on every card. On success: stage→task, then run pipeline-impl (omp / goal-driven-impl-
claude, capable-local OK) to make tests/grid_tick.rs green on a feat/grid-tick branch.
On failure: attempts++; >=3 ⇒ blocked ⇒ run pipeline-hunt.
<<< END
