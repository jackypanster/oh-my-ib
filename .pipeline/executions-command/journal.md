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

## seq=3 · 2026-07-01T16:39:48Z · task→impl · completed · by=claude-opus-4-8(claude-code)
done:   ONE atomic card. Froze the red spec tests/executions_command.rs in a SEPARATE freeze commit
        374ea2f (spec-rev): 2 black-box (--help lists `executions`; `executions --help` ok) + 6 pure-seam
        `merge_executions` cases (matched join → numeric commission/currency/realized_pnl; missing
        commission → 3x null; realized_pnl sentinel 1.7976931348623157e308 / None → null via pnl_number;
        order preserved; side "BOT"/"SLD" verbatim; orphan commission dropped; empty → []). Verified
        genuinely RED for the right reason: `cargo build` GREEN (lib+bin), `cargo test --test
        executions_command` fails with E0432 unresolved `oh_my_ib::ib::{merge_executions,ExecRow,
        CommissionRow}` (goes green when impl adds the seam + Executions subcommand). Wrote card 01
        (spec-rev=374ea2f, status=todo, verify=[cargo build, cargo test --test executions_command],
        spec-paths=tests/executions_command.rs, impl-paths=src/ib/executions.rs+mod.rs+cli.rs+main.rs,
        disjoint). current.json stage=task, full-verify=[cargo build, cargo test].
output: tests/executions_command.rs (freeze commit 374ea2f), .pipeline/executions-command/tasks/01.md,
        .pipeline/current.json, .pipeline/executions-command/journal.md
--- handoff ---
>>> NEXT
Run pipeline-impl on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none  card=01
Model: frontier SOTA required — operator assigns the bot; the pipeline can't verify the model.
First: git pull --rebase; no .env in this repo (config at ~/.config/oh-my-ib/config.toml, not needed to build/test).
Read for context (before acting):
  - oh-my-ib/AGENTS.md — repo conventions (agent-first, hard safety rules, verify model). Read FIRST.
  - .pipeline/executions-command/tasks/01.md — the card (scope, impl-paths, near-complete impl guidance)
  - .pipeline/executions-command/arch.md — component boundaries, ibapi surface, data flow, frozen surface
  - .pipeline/executions-command/CONTEXT.md — execution/fill/commission/exec_id glossary
  - .pipeline/executions-command/docs/adr/0008-executions-drain-and-commission-join.md — drain-to-End + join proof
  - src/ib/orders.rs + src/ib/positions.rs — the drain-to-End iter_data() pattern to mirror
  - src/ib/pnl.rs — pnl_number (REUSE for realized_pnl; already re-exported in mod.rs)
Your task (concrete, numbered):
  1. Branch feat/executions-command off main. Implement card 01 in impl-paths ONLY:
     src/ib/executions.rs (ExecRow/CommissionRow plain structs + pure merge_executions seam + gateway
     executions(cfg)); src/ib/mod.rs (`mod executions;` + `pub use executions::{executions,
     merge_executions, ExecRow, CommissionRow};`); src/cli.rs (Command::Executions, no args);
     src/main.rs (dispatch Command::Executions => ib::executions(&config)).
  2. Make verify GREEN: `cargo build` && `cargo test --test executions_command`. Also keep the whole
     suite + clippy clean: `cargo test` (full-verify) and `cargo clippy --all-targets -- -D warnings`.
  3. Open a PR feat/executions-command -> main. Do NOT merge (only pipeline-review merges, after a human go).
Feature gotchas (project-specific traps you MUST honor):
  - DRAIN TO END via `for item in sub.iter_data()` — ExecutionDataEnd → EndOfStream (ADR 0008). Do NOT
    take-first (that's reqPnL/ADR 0007); do NOT loop without End (there IS an End here).
  - JOIN exec↔commission by exec_id in merge_executions; missing commission → 3x null (NOT error);
    orphan commission → drop (no phantom row). Best-effort by stream position (Tiger may omit/reorder).
  - realized_pnl MUST reuse super::pnl_number (do NOT re-implement the sentinel filter).
  - side = d.execution.side.as_str().to_string() → "BOT"/"SLD" (NOT Debug). time = raw IB string.
    Empty result = exit 0 {account, executions:[]}.
  - ExecRow/CommissionRow field lists are FROZEN by the test (struct-literal construction) — match them
    EXACTLY (see card 01). Changing a field name/type breaks the frozen spec → re-route to task, do NOT edit the test.
  - Do NOT edit tests/executions_command.rs (spec-paths) or any other frozen spec. spec ∩ impl = ∅.
  - reqExecutions is current-day only; no filter flags this card (ExecutionFilter default except account_code).
Done when: card 01 verify green + full-verify green + clippy clean; PR opened feat/executions-command -> main;
card status=in-progress→review; journal appended; then run pipeline-review.
On failure: attempts++; >=3 ⇒ blocked ⇒ run pipeline-hunt.
<<< END

## seq=5 · 2026-07-01T16:50:52Z · impl→review · completed · by=claude-opus-4-8(claude-code)
done:   Reviewed PR #8 (head 2e12854, feat/executions-command). FREEZE GATE PASS: git diff 374ea2f..2e12854
        -- tests/executions_command.rs empty (frozen spec untouched); no other spec touched. Semantic
        review (check): PR surface = only the 4 card impl-paths (spec ∩ impl = ∅); merge_executions join
        matches the frozen contract (order preserved, missing→null, orphan dropped, realized_pnl via
        pnl_number); gateway executions() drains to End via iter_data (ADR 0008) — verified in ibapi source
        that ExecutionDataEnd→EndOfStream→NextAction::Return(None) (sync.rs:171), so the drain terminates
        cleanly and never spuriously errors (cannot hang; opposite of reqPnL/ADR 0007); CommissionReport
        routed ByExecutionId correlates by exec_id; read-only (no place/modify/cancel, no OMI_ALLOW_LIVE);
        output.rs untouched (table free). FULL-SUITE GATE PASS on 2e12854: cargo build + cargo test (whole
        suite 35 tests, executions_command 8/8, no regressions) + clippy --all-targets -D warnings clean.
        Wrote review-01.md (verdict: approve). AWAITING EXPLICIT HUMAN MERGE CONFIRM — only pipeline-review
        merges, never without a go.
output: .pipeline/executions-command/reviews/review-01.md, .pipeline/executions-command/journal.md
--- handoff ---
APPROVED, awaiting human confirm. On operator "go": squash-merge PR #8 via gh, delete feat/executions-
command, set card 01 status=done, current.json stage=done, append the final review→done journal entry,
push main. Post-merge (operator, not a merge gate): live acceptance `omi --live executions` once the Tiger
gateway reopens on :4001. On rejection: attempts++, append fixes to card 01, flip status=todo (>=3 ⇒
blocked), route pipeline-impl (or pipeline-hunt).
<<< END

## seq=4 · 2026-07-02T00:00:00Z · impl→review · completed · by=claude-sonnet-5(claude-code)
done:   Card 01 implemented on feat/executions-command (cut from main @ 75dc44e). New src/ib/executions.rs:
        ExecRow/CommissionRow plain rows (fields match the frozen test's struct literals exactly);
        pure merge_executions(execs, comms) JOIN seam — HashMap<exec_id,&CommissionRow> index, per-exec
        passthrough + joined/null commission fields, orphan commissions dropped, order preserved; gateway
        executions(cfg) mirrors orders.rs/positions.rs drain-to-End via `for item in sub.iter_data()`
        (ADR 0008 — NOT the reqPnL take-first of ADR 0007). realized_pnl reuses super::pnl_number verbatim.
        side = d.execution.side.as_str().to_string() ("BOT"/"SLD"). d.contract.symbol is ibapi's `Symbol`
        newtype (not String) — needed `.to_string()` (only surprise vs the card's guidance). Wired
        mod.rs (`mod executions;` + re-exports), cli.rs (Command::Executions, no args), main.rs (dispatch).
        Did NOT touch tests/executions_command.rs (spec-paths) or any other frozen spec.
        Verify green: cargo build; cargo test --test executions_command (8/8). Also full-verify green:
        cargo test (whole suite, all files incl. executions_command 8/8); cargo clippy --all-targets
        -- -D warnings (clean, zero warnings). Pushed feat/executions-command, opened PR #8 via `gh`
        (forge token present — gh auth status confirmed jackypanster). Card 01 -> status=review on main;
        current.json.pr set, stage=impl. This is the feature's ONLY card, so ALL cards are now `review`
        — routing to pipeline-review (feature-complete), not back to pipeline-impl.
output: https://github.com/jackypanster/oh-my-ib/pull/8, src/ib/executions.rs, src/ib/mod.rs, src/cli.rs,
        src/main.rs (on feat/executions-command @ 2e12854), .pipeline/executions-command/tasks/01.md,
        .pipeline/current.json, .pipeline/executions-command/journal.md
--- handoff ---
>>> NEXT
Run pipeline-review on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git base=main branch=feat/executions-command pr=https://github.com/jackypanster/oh-my-ib/pull/8
Model: frontier SOTA required — operator assigns the bot; the pipeline can't verify the model.
First: git pull --rebase; no .env in this repo (config at ~/.config/oh-my-ib/config.toml, not needed to build/test).
Read for context (before acting):
  - oh-my-ib/AGENTS.md — repo conventions (agent-first, hard safety rules, verify model). Read FIRST.
  - .pipeline/executions-command/tasks/01.md — the card (spec-rev, spec-paths, impl-paths, freeze coverage section)
  - .pipeline/executions-command/arch.md, CONTEXT.md, docs/adr/0008-executions-drain-and-commission-join.md
  - PR #8 diff (feat/executions-command vs main) — src/ib/executions.rs, src/ib/mod.rs, src/cli.rs, src/main.rs
Your task (concrete, numbered):
  1. Freeze gate FIRST (deterministic, two-commit diff): `git diff 374ea2f <branch-tip> -- tests/executions_command.rs`
     MUST be empty (spec untouched). Non-empty ⇒ reject, attempts++, route to pipeline-impl (or hunt at >=3).
  2. Read-review what the freeze coverage section (card 01) flags as NOT frozen: `client.executions()` wiring,
     the drain-to-End `iter_data()` loop (must NOT be take-first, must NOT loop without an End — ADR 0008),
     the ibapi-item -> row extraction (exec_id/order_id/perm_id/time/symbol/conid/side/shares/price/
     cumulative_qty/avg_price/exchange field mapping), resolve_account reuse, the {account, executions:[...]}
     assembly, and --format table (should be free via generic render_table).
  3. Run the full-suite gate on the branch HEAD: current.json.full-verify = ["cargo build", "cargo test"].
     Both must be green. Also re-run `cargo clippy --all-targets -- -D warnings` (clean expected).
  4. Semantic check: side is the wire string via ExecutionSide::as_str() (NOT Debug); realized_pnl goes
     through pnl_number (no reimplemented sentinel filter); orphan commission dropped, not phantom-rowed;
     missing commission -> null triple, not an error; ExecRow/CommissionRow field lists match card 01 exactly.
  5. If clean: write reviews/review-01.md, flip card 01 status -> done, advance current.json.stage -> review,
     append journal seq=5, commit to main. Then get an explicit human confirm before squash-merging PR #8
     (only pipeline-review merges). After merge: current.json.stage -> done.
Feature gotchas (project-specific traps the next node MUST know):
  - This is a binary+lib crate: gateway wiring can't be frozen offline — review-by-reading covers the
    gaps the freeze gate can't (see card 01 Freeze coverage section) instead of demanding more tests.
  - Live acceptance (`omi --live executions` after a day with fills) is BLOCKED until the operator reopens
    the Tiger gateway on :4001 — do NOT block the merge gate on it; it's a separate operator-run check.
  - Do NOT touch other frozen specs (phase1 cli_contract, tz, connect-retry, quote, pnl, agents-md, slim-claude-md).
  - Only this card on this feature — once it's done, current.json.stage should read review before human-confirmed merge, then done after merge.
Done when: review-01.md written, freeze gate + full-suite gate confirmed green, card 01 -> done, human has
explicitly confirmed the merge, PR #8 squash-merged into main, current.json.stage -> done.
On failure: attempts++; >=3 ⇒ blocked ⇒ run pipeline-hunt (route to the offending card or a feature-level
reviews/integration-NN.md report per CONTRACT, never a blind retry).
<<< END
