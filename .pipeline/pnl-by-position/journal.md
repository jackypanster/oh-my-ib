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
