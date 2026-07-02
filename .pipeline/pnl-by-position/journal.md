# journal — pnl-by-position

## seq=1 · 2026-07-02T16:00:00Z · prd→arch · completed · by=claude-fable-5(claude-code)
done:   New feature: `omi pnl-by-position` — per-position Daily/Unrealized/Realized PnL, the last
        read-only gap (`positions` = inception unrealized only; `pnl` = account-level only; its PRD
        pre-deferred this exact feature). Chosen in a /think ROI pass over `executions-filters` (new
        capability vs client-side-replicable convenience; Phase 2 orders stays deferred). Verified
        against ibapi-3.1.0 source: `pnl_single(&account, ContractId, Option<&ModelCode>) ->
        Subscription<PnLSingle>` (accounts/sync.rs:159); `PnLSingle{position,daily_pnl,unrealized_pnl,
        realized_pnl,value}` all bare f64 → sentinel routes through existing `pnl_number`;
        StreamDecoder<PnLSingle> = [PnLSingle, Error], NO End marker → ADR 0007 take-first is binding.
        Operator locked (HITL): D1 new flat subcommand (not a --by-position flag on pnl); D2 all
        positions, no filters; D3 MERGE GATE = live `omi --live pnl` acceptance must pass BEFORE this
        PR merges (reqPnL-family support on Tiger unverified; gateway currently closed).
        Decision-complete PRD written; current.json repointed to pnl-by-position @ prd.
output: .pipeline/pnl-by-position/PRD.md, .pipeline/current.json, .pipeline/pnl-by-position/journal.md
--- handoff ---
>>> NEXT
Run pipeline-arch on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
Model: frontier SOTA required — operator assigns the bot; the pipeline can't verify the model.
First: git pull --rebase; no .env in this repo (config lives at ~/.config/oh-my-ib/config.toml, not needed for arch).
Read for context (before acting):
  - oh-my-ib/AGENTS.md — repo conventions (agent-first authoring, hard safety rules, verify model). Read FIRST.
  - .pipeline/pnl-by-position/PRD.md — what (this feature; D1-D6 are locked)
  - .pipeline/pnl-command/docs/adr/0007-pnl-take-first-reading.md — BINDING: take-first for markerless
    PnL streams; its Consequences pre-commit this feature
  - src/ib/pnl.rs — the pnl_number sentinel seam to REUSE + the take-first shape to mirror
  - src/ib/positions.rs — the account_updates conid/symbol discovery pattern (drain-to-End)
  - src/ib/executions.rs + tests/executions_command.rs — the newest pure-seam freeze pattern (merge_executions) to mirror
  - tests/cli_contract.rs — the black-box CLI freeze style to extend for `pnl-by-position`
Your task (concrete, numbered):
  1. Lock the design in arch.md: new gateway module (src/ib/pnl_by_position.rs or extend pnl.rs — decide
     and justify) with pnl_by_position(cfg); a PURE ibapi-free row/shape seam (mirror merge_executions)
     so the frozen test needs no gateway; cli.rs subcommand `pnl-by-position` (clap kebab-case name);
     main.rs dispatch; mod.rs export. Fix exact ibapi type paths (ContractId/ModelCode — where do the
     newtypes live, how does i32 conid from PortfolioValue convert).
  2. RESOLVE THE SWEEP SHAPE (the one real design risk): account_updates drain-to-End → drop → N
     sequential pnl_single take-first reads on the SAME client. Confirm request-id isolation makes the
     interleaving safe (known Tiger EAGAIN quirk lives at connect, src/ib/mod.rs — does it also bite
     between subscriptions mid-session?). DECIDE error semantics when one conid's pnl_single fails
     mid-sweep (fail-fast whole command vs per-row degradation) — PRD is silent, arch must decide and
     record an ADR. Also decide qty==0 row handling implementation (PRD D6: query them).
  3. Specify the frozen test surface: binary crate (no lib.rs) — black-box CLI (`--help` lists
     `pnl-by-position`; `pnl-by-position --help` ok) + pure seam tests (row shaping through pnl_number:
     sentinel 1.7976931348623157e308 → null, finite → number; empty discovery → "by_position":[];
     ordering preserved). Gateway wiring NOT frozen (Freeze coverage section per CONTRACT).
  4. Specify the JSON contract exactly as PRD success-criterion 1: {"account","by_position":[{conid,
     symbol,position,daily_pnl,unrealized_pnl,realized_pnl,value}]} snake_case, stable key order not
     required but keys frozen.
Feature gotchas (project-specific traps the next node MUST know):
  - reqPnLSingle stream is MARKERLESS ([PnLSingle, Error], ibapi-3.1.0 stream_decoders/mod.rs:53-58) —
    take-first per ADR 0007, NEVER drain-to-End; a drain loop hangs forever.
  - PnLSingle fields are bare f64, NOT Option (accounts/mod.rs:172) — f64::MAX sentinel arrives as a
    value; route EVERY PnL field through pnl_number (src/ib/pnl.rs), including `value`? decide: `value`
    is market value, not PnL — arch decides if it also gets sentinel treatment and records it.
  - conid discovery = account_updates PortfolioValue (positions.rs), NOT client.positions() (lacks
    valuation + this repo's documented choice, see positions.rs header).
  - MERGE GATE (PRD D3): the eventual PR must NOT merge until the operator live-accepts
    `omi --live pnl` first (Tiger gateway currently CLOSED — the gate waits for the operator).
  - Binary crate → freeze covers only the CLI/black-box + pure seam; review reads the gateway wiring
    by hand (record in Freeze coverage).
Done when: arch.md + CONTEXT.md + docs/adr/* committed on main + pushed, journal seq=2 appended,
current.json.stage=arch. On success: run pipeline-task.
On failure: attempts++; >=3 ⇒ blocked ⇒ run pipeline-hunt.
<<< END

## seq=2 · 2026-07-02T16:10:14Z · arch→task · completed · by=claude-fable-5(claude-code)
done:   Architecture locked, all design questions resolved CODE-FIRST (zero human questions needed).
        Shape: new src/ib/pnl_by_position.rs = ibapi-free PnlSingleRow + pure seam
        shape_pnl_by_position(Vec<PnlSingleRow>) -> Value (mirror merge_executions) + gateway fn
        pnl_by_position(cfg) doing discovery (account_updates drain-to-End, positions.rs pattern, drop
        → cancel) then N sequential pnl_single take-first reads in discovery order; Command::PnlByPosition
        (clap kebab-cases to pnl-by-position); dispatch in main.rs; re-exports in ib/mod.rs. Interleaving
        proven safe in crate source: account_updates = shared-channel routing, pnl_single = request-id
        routing (disjoint domains, sequential phases). ADR 0009: fail-fast (no partial sweep — partial is
        indistinguishable from complete to the agent); sentinel routing incl. defensive `value`; blocking
        next_data() default with next_timeout fallback. CORRECTIONS vs seq=1 handoff: crate is lib+bin
        (ADR 0006) NOT "binary crate"; ADR 0007 file is 0007-pnl-take-first-unbounded-stream.md.
output: .pipeline/pnl-by-position/arch.md, .pipeline/pnl-by-position/CONTEXT.md,
        .pipeline/pnl-by-position/docs/adr/0009-pnl-by-position-sweep.md, .pipeline/current.json
--- handoff ---
>>> NEXT
Run pipeline-task on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
Model: frontier SOTA required — operator assigns the bot; the pipeline can't verify the model.
First: git pull --rebase; no .env in this repo (config lives at ~/.config/oh-my-ib/config.toml, not needed for task).
Read for context (before acting):
  - oh-my-ib/AGENTS.md — repo conventions (agent-first authoring, hard safety rules). Read FIRST.
  - .pipeline/pnl-by-position/PRD.md — what (D1-D6 locked)
  - .pipeline/pnl-by-position/arch.md — the locked shape; its §Freeze coverage is your card input
  - .pipeline/pnl-by-position/CONTEXT.md + docs/adr/0009-pnl-by-position-sweep.md — binding terms + stream decision
  - .pipeline/executions-command/tasks/01.md + tests/executions_command.rs — the card + frozen-test model to MIRROR
Your task (concrete, numbered):
  1. ONE card (mirror executions card 01): card 01 = whole feature (module + seam + CLI variant +
     dispatch). Arch fixed the components; no split (one command, one module, one seam).
  2. FREEZE COMMIT (touches ONLY spec-paths, ONE commit): write tests/pnl_by_position_command.rs per
     arch.md §Freeze coverage — black-box (`omi --help` lists `pnl-by-position`;
     `omi pnl-by-position --help` exit 0) + pure seam via
     `use oh_my_ib::ib::{shape_pnl_by_position, PnlSingleRow}` (finite pass-through; sentinel
     1.7976931348623157e308 → null in daily/unrealized/realized/value; NAN/INFINITY → null;
     conid/symbol/position raw; row order preserved; empty Vec → []; exact 7-key set per row).
     Mirror executions_command.rs header style (FROZEN SPEC banner + explicit NOT-frozen list).
     Expected RED = the card-scoped runner FAILS at spec-rev (unresolved lib import + missing
     subcommand — the sanctioned red shape, exactly like executions card 01).
  3. RECORD COMMIT (metadata only, never spec-paths): tasks/01.md frontmatter — status: todo,
     attempts: 0, spec-rev: <freeze commit sha>, spec-paths: [tests/pnl_by_position_command.rs],
     impl-paths: [src/ib/pnl_by_position.rs, src/cli.rs, src/main.rs, src/ib/mod.rs],
     verify: `cargo test --test pnl_by_position_command` (card-scoped, NEVER full suite).
     Include ## Freeze coverage (frozen = CLI black-box + pure seam; review must READ the gateway fn
     + hold PRD D3 live gate). Advance current.json.stage=task (full-verify stays
     ["cargo build","cargo test"]). Append journal seq=3. Commit once; push BOTH commits.
  4. Assert spec-paths ∩ impl-paths = ∅ before committing.
Feature gotchas (traps the next node MUST know):
  - Crate is lib+bin (ADR 0006): the frozen test imports oh_my_ib::ib::... — do NOT write it as
    binary-only black-box; the pure-seam half needs the lib import.
  - Write ZERO src code — task authors the red test + card metadata only.
  - reqPnLSingle is MARKERLESS (take-first, ADR 0009) — nothing gateway-touching belongs in the
    frozen file; the seam is pure by design so the test needs no ibapi import.
  - MERGE GATE (PRD D3): carries through every later handoff — PR must not merge until operator
    live-accepts `omi --live pnl` first (Tiger gateway currently CLOSED).
Done when: freeze commit + record commit pushed to main, journal seq=3 appended, card 01 = todo.
On success: run pipeline-impl. On failure: attempts++; >=3 ⇒ blocked ⇒ run pipeline-hunt.
<<< END

## seq=3 · 2026-07-02T16:17:07Z · task→impl · completed · by=claude-fable-5(claude-code)
done:   ONE card (arch pre-fixed the split). Freeze commit 367d671 = spec-rev: tests/
        pnl_by_position_command.rs — black-box CLI (help lists pnl-by-position; subcommand --help ok)
        + pure seam via oh_my_ib::ib lib import (finite pass-through; sentinel → null in all 4 money
        fields; NAN/±INF → null; conid/symbol/position raw; qty==0 row EMITTED not filtered (PRD D6);
        exact 7-key set; order preserved; [] on empty). RED verified at spec-rev: card runner fails
        with E0432 unresolved import (the sanctioned red, same shape as executions card 01);
        `cargo build` stays green. Record commit: tasks/01.md (status=todo, attempts=0, verify
        card-scoped `cargo test --test pnl_by_position_command`, spec∩impl=∅ asserted),
        current.json.stage=task, full-verify unchanged ["cargo build","cargo test"]. NOTE: trunk
        full suite is now RED by design until the impl PR merges (CONTRACT §State authority).
output: tests/pnl_by_position_command.rs (spec-rev 367d671334311f428fb917a7a54fc9b84b8289f8),
        .pipeline/pnl-by-position/tasks/01.md, .pipeline/current.json
--- handoff ---
>>> NEXT
Run pipeline-impl on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
Model: capable-local OK (impl only) — operator assigns the bot; the pipeline can't verify the model.
First: git pull --rebase; no .env in this repo (config lives at ~/.config/oh-my-ib/config.toml, not needed offline).
Read for context (before acting):
  - oh-my-ib/AGENTS.md — repo conventions (agent-first authoring, hard safety rules). Read FIRST.
  - .pipeline/pnl-by-position/tasks/01.md — THE card: exact scope, verify, freeze coverage
  - .pipeline/pnl-by-position/arch.md — component boundaries + step-by-step gateway fn shape
  - .pipeline/pnl-by-position/CONTEXT.md + docs/adr/0009-pnl-by-position-sweep.md — binding: take-first, fail-fast, sentinel routing
  - src/ib/pnl.rs (pnl_number seam + take-first model), src/ib/positions.rs (discovery model),
    src/ib/executions.rs (newest module/seam style to mirror)
Your task (concrete, numbered):
  1. Pick card 01 (oldest todo): set status=in-progress (journal it), cut feat/pnl-by-position from trunk.
  2. Make the card verify green — `cargo build` + `cargo test --test pnl_by_position_command` — by
     writing ONLY impl-paths: src/ib/pnl_by_position.rs (PnlSingleRow + shape_pnl_by_position +
     pnl_by_position per the card's step-by-step), src/cli.rs (PnlByPosition variant), src/main.rs
     (dispatch arm), src/ib/mod.rs (mod + pub use).
  3. Also run `cargo clippy --all-targets -- -D warnings` (repo verify convention, AGENTS.md).
  4. Push feat/pnl-by-position; open the PR (gh pr create, base main); card status=review; append
     journal seq=4; commit metadata to trunk; push.
  5. NEVER create/modify/delete tests/pnl_by_position_command.rs or ANY tests/** file (all frozen).
Feature gotchas (traps the next node MUST know):
  - TAKE-FIRST per pnl_single subscription: next_data() once, then drop. NEVER iter_data()/drain —
    reqPnLSingle is markerless (no End); a drain loop hangs forever (ADR 0009, ADR 0007).
  - FAIL-FAST: Some(Err)/None from ANY read → AppError::data naming the failing conid; never emit a
    partial by_position array (ADR 0009 — partial is indistinguishable from complete to the agent).
  - symbol comes from DISCOVERY (PnLSingle carries no contract identity); position/value/PnL fields
    come from the PnLSingle reading (fresher than the portfolio snapshot).
  - All 4 money fields (daily/unrealized/realized/value) through pnl_number(Some(x)); conid/symbol/
    position raw; never filter qty==0 rows (frozen test asserts this).
  - ContractId is a newtype: ibapi::accounts::types::ContractId, From<i32> exists.
  - clap derive kebab-cases PnlByPosition → pnl-by-position automatically; no rename attribute.
  - Drop the account_updates subscription BEFORE the sweep (explicit drop — Drop sends cancel).
  - Trunk full suite is RED at spec-rev by design; only the card-scoped runner must go green on your
    branch (plus build + clippy). Review runs the full suite on the PR head (full-verify).
  - MERGE GATE (PRD D3): review must NOT merge until operator live-accepts `omi --live pnl` first
    (Tiger gateway currently CLOSED) — carry this line into your handoff to review.
Done when: card verify + clippy green on feat/pnl-by-position, PR open, card status=review,
journal seq=4 appended + pushed. On success: run pipeline-review.
On failure: attempts++; >=3 ⇒ blocked ⇒ run pipeline-hunt.
<<< END
