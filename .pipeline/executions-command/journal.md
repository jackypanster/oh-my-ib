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

## seq=2 · 2026-07-01T16:35:00Z · arch→task · completed · by=claude-opus-4-8(claude-code)
done:   Locked design (all PRD claims code-verified against the repo AND ibapi-3.1.0 source). New
        src/ib/executions.rs = gateway fn `executions(cfg)` + pure frozen JOIN seam
        `merge_executions(Vec<ExecRow>, Vec<CommissionRow>) -> Value` over plain ibapi-free structs
        (mirrors pnl.rs/account.rs split). RESOLVED THE DRAIN SHAPE (the one design risk): StreamDecoder
        <Executions> maps ExecutionDataEnd -> Error::EndOfStream (stream_decoders.rs:78), so iter_data
        DRAINS TO END (orders/positions shape) — the OPPOSITE of reqPnL/ADR 0007, no take-first, no hang.
        CommissionReport carries no request_id/order_id; ibapi routes it ByExecutionId (routing.rs:132)
        via an exec_id->subscription mapping stored when the matching ExecutionData was routed
        (routing.rs:129) — so exec + commission arrive interleaved on ONE subscription, JOIN key = exec_id.
        Best-effort join: only commissions arriving before End are joined; missing -> null (graceful, key
        for the Tiger gateway which may omit/reorder commissions). realized_pnl reuses pnl_number.
        side=BOT/SLD via ExecutionSide::as_str; time raw; empty=exit 0 []. render_table is generic -> table
        free. Wrote arch.md, CONTEXT.md (execution/fill/commission/exec_id glossary), ADR 0008. Reuses
        Phase 1 ADRs 0001-0006 + pnl_number seam.
output: .pipeline/executions-command/arch.md, .pipeline/executions-command/CONTEXT.md,
        .pipeline/executions-command/docs/adr/0008-executions-drain-and-commission-join.md,
        .pipeline/current.json
--- handoff ---
>>> NEXT
Run pipeline-task on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
Model: frontier SOTA required — operator assigns the bot; the pipeline can't verify the model.
First: git pull --rebase; no .env in this repo (config at ~/.config/oh-my-ib/config.toml, not needed).
Read for context (before acting):
  - oh-my-ib/AGENTS.md — repo conventions (agent-first, hard safety rules, verify model). Read FIRST.
  - .pipeline/executions-command/PRD.md — what
  - .pipeline/executions-command/arch.md — how (component boundaries, data flow, ibapi surface, frozen surface)
  - .pipeline/executions-command/CONTEXT.md — execution/fill/commission/exec_id glossary
  - .pipeline/executions-command/docs/adr/0008-executions-drain-and-commission-join.md — the drain+join decision
  - tests/cli_contract.rs (black-box style to extend) + tests/pnl_command.rs (pure-seam freeze pattern to mirror)
Your task (concrete, numbered):
  1. ONE card (tasks/01.md). Freeze ALL of this feature's red tests in tests/executions_command.rs (offline):
       a. black-box (assert_cmd, mirror cli_contract.rs): `omi --help` stdout contains "executions";
          `omi executions --help` exits 0.
       b. pure seam (NO ibapi import — merge_executions over plain ExecRow/CommissionRow, both pub w/ pub fields):
          - matched join (exec + commission, same exec_id) -> object with numeric commission, string
            commission_currency, numeric realized_pnl.
          - unmatched exec (no commission) -> commission/commission_currency/realized_pnl == Value::Null.
          - realized_pnl sentinel: CommissionRow{realized_pnl:Some(1.7976931348623157e308)} -> null;
            realized_pnl:None -> null.
          - order preserved across >=2 execs; side string ("BOT"/"SLD") passes through verbatim.
          - orphan commission (exec_id matches no exec) -> dropped (no phantom row); merge_executions([],[]) -> [].
     The red test compiles against the PUBLIC seam signature (merge_executions + ExecRow/CommissionRow field
     lists per arch.md) but fails/does-not-exist until impl. Do NOT write gateway wiring tests.
  2. Card spec-paths (frozen) = tests/executions_command.rs. Card impl-paths = src/ib/executions.rs,
     src/ib/mod.rs, src/cli.rs, src/main.rs. Verify commands: cargo build; cargo test --test executions_command;
     cargo clippy --all-targets -- -D warnings.
  3. Card acceptance (operator, live, not a merge gate): `omi --live executions` after a day with fills.
Feature gotchas (project-specific traps the next node MUST know):
  - Drain to End via iter_data() (ExecutionDataEnd -> EndOfStream). Do NOT take-first (that's reqPnL/ADR 0007);
    do NOT loop without End (there IS an End here). ADR 0008 has the proof.
  - JOIN exec<->commission by exec_id in merge_executions; missing commission -> null fields (NOT error);
    orphan commission -> drop. Best-effort by stream position (Tiger may omit/reorder commissions).
  - realized_pnl MUST reuse src/ib/pnl.rs::pnl_number (already re-exported in mod.rs). Do NOT re-implement.
  - side = "BOT"/"SLD" (ExecutionSide::as_str), not Debug. time = raw IB string. Empty = exit 0 [].
  - reqExecutions is CURRENT-DAY ONLY. No filter flags this card (ExecutionFilter default except account_code).
  - Binary+lib crate: freeze only the black-box CLI + the pure merge_executions seam; gateway wiring is
    reviewed-by-reading + live acceptance.
  - Do NOT touch other frozen specs (phase1 cli_contract, tz, connect-retry, quote, pnl, agents-md, slim-claude-md).
Done when: tasks/01.md written with the frozen red test committed to trunk (spec-rev), current.json stage=task;
then run pipeline-impl.
On failure: attempts++; >=3 ⇒ blocked ⇒ run pipeline-hunt.
<<< END
