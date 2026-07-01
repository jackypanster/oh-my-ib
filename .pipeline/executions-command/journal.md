# journal — executions-command

## seq=1 · 2026-07-01T16:27:24Z · prd→arch · completed · by=claude-opus-4-8(claude-code)
done:   New feature: `omi executions` — the account's current-day executions (fills), the one missing
        piece of the order lifecycle (`orders` = working only; `positions.realized_pnl` = cumulative, not
        an itemized fill log). Chosen in a prior /think ROI pass over `completed_orders` (fill-level richer:
        price + commission vs order-level terminal states that overlap `orders`). Verified against ibapi
        3.1.0 sync: `client.executions(ExecutionFilter) -> Subscription<Executions>` (src/orders/sync.rs:144,
        read-only); `Executions` interleaves `ExecutionData` + `CommissionReport`, joined by `exec_id`;
        `Execution.side` -> `ExecutionSide::as_str` = "BOT"/"SLD"; `CommissionReport.realized_pnl:
        Option<f64>` reuses the existing `pnl_number` sentinel seam. Operator locked (HITL): command name
        `executions`; card 01 = minimal, NO filter flags (account_code server-side scope only; --symbol/
        --side deferred to a future `executions-filters` card). Decision-complete PRD written; current.json
        repointed to executions-command @ prd.
output: .pipeline/executions-command/PRD.md, .pipeline/current.json, .pipeline/executions-command/journal.md
--- handoff ---
>>> NEXT
Run pipeline-arch on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
Model: frontier SOTA required — operator assigns the bot; the pipeline can't verify the model.
First: git pull --rebase; no .env in this repo (config lives at ~/.config/oh-my-ib/config.toml, not needed for arch).
Read for context (before acting):
  - oh-my-ib/AGENTS.md — repo conventions (agent-first authoring, hard safety rules, verify model). Read FIRST.
  - .pipeline/executions-command/PRD.md — what (this feature)
  - src/ib/orders.rs + src/ib/positions.rs — the drain-to-End subscription pattern (iter_data → match enum → push JSON)
  - src/ib/pnl.rs — the `pnl_number` sentinel seam to REUSE for realized_pnl; ADR 0007 (unbounded-stream trap) is the model for the drain question below
  - src/ib/quote.rs + tests/quote_ticks.rs — the pure-seam freeze pattern (quote_price_tick) to mirror for merge_executions
  - tests/cli_contract.rs — the black-box CLI freeze style to extend for `executions`
Your task (concrete, numbered):
  1. Lock the design in arch.md: new src/ib/executions.rs with `executions(cfg)` (gateway) + the pure
     `merge_executions(Vec<ExecRow>, Vec<CommissionRow>) -> Value` JOIN seam; cli.rs Command::Executions
     (no args); main.rs dispatch; mod.rs `pub use executions::{executions, merge_executions}`. Fix the exact
     plain `ExecRow`/`CommissionRow` field lists (ibapi-free, so the seam test needs NO ibapi import).
  2. RESOLVE THE DRAIN SHAPE (the ADR-0007 analog — this is the one real design risk): read ibapi 3.1.0
     src/orders/sync.rs `executions()` + how `Subscription<Executions>` terminates. Confirm whether
     iter_data drains to an End (like orders/positions) or needs another strategy, AND whether
     `CommissionReport` items are delivered BEFORE termination (so the join has all commissions). Record
     an ADR if the drain shape is non-obvious. Map ibapi paths: `ibapi::orders::{Executions, ExecutionData,
     CommissionReport, ExecutionFilter, ExecutionSide}`; `Execution.side.as_str()`; contract.symbol /
     contract.contract_id → symbol/conid.
  3. Specify the frozen test surface: tests/executions_command.rs (offline) = black-box (`--help` lists
     `executions`, `executions --help` ok) + pure seam (matched join → numeric commission fields; unmatched
     exec → null commission fields; realized_pnl sentinel 1.7976931348623157e308 / None → null via pnl_number;
     order preserved; orphan commission dropped; empty input → []). Gateway wiring NOT frozen.
  4. Specify JSON contract exactly as PRD success-criterion 1 (snake_case): {account, executions:[{exec_id,
     order_id, perm_id, time, symbol, conid, side, shares, price, cumulative_qty, avg_price, exchange,
     commission, commission_currency, realized_pnl}]}. side = "BOT"/"SLD"; time raw; empty = exit 0 [].
Feature gotchas (project-specific traps the next node MUST know):
  - reqExecutions is CURRENT-DAY ONLY by API — no historical/multi-day paging; do not design for it.
  - The subscription interleaves ExecutionData + CommissionReport; JOIN by exec_id. Missing commission → null
    fields (NOT an error). Orphan commission (no matching exec) → drop, no phantom row.
  - realized_pnl MUST go through the existing pnl_number seam (IB sentinel 1.7e308 / non-finite / None → null).
    Do NOT re-implement it; reuse src/ib/pnl.rs::pnl_number.
  - side MUST be the wire string "BOT"/"SLD" (ExecutionSide::as_str), not Debug.
  - Empty result is SUCCESS (exit 0, executions:[]), not an error.
  - Binary+lib crate: gateway behavior can't be frozen offline; freeze only the black-box CLI + pure seam.
  - Do NOT touch other frozen specs (phase1 cli_contract, tz, connect-retry, quote, pnl, agents-md, slim-claude-md).
  - Live acceptance is BLOCKED until the operator reopens the Tiger gateway on :4001 — offline gates still run.
    Tiger may stub executions (→ []) or omit commission reports (→ null commission fields); both degrade gracefully.
Done when: arch.md (+ CONTEXT.md if the glossary needs execution/commission terms; ADR if the drain shape
warrants one) committed to trunk; then run pipeline-task.
On failure: attempts++; >=3 ⇒ blocked ⇒ run pipeline-hunt.
<<< END
