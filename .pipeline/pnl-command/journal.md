# journal — pnl-command

## seq=1 · 2026-06-30T15:56:05Z · prd→arch · completed · by=claude-opus-4-8(claude-code)
done:   New feature: `omi pnl` — account-level Daily/Unrealized/Realized PnL, the one missing datum in
        the monitoring loop (`account` has only static balances; agent can't derive Daily PnL). Verified
        ibapi 3.1.0 sync `client.pnl(&account, None)` → Subscription<PnL{daily_pnl:f64, unrealized_pnl,
        realized_pnl:Option<f64>}>. Operator chose sentinel→null (A): IB's Double.MAX_VALUE 1.7e308 "no
        value" marker maps to JSON null via a pure `pnl_number` seam. Read-only, no write gate. Decision-
        complete PRD written; current.json repointed.
output: .pipeline/pnl-command/PRD.md, .pipeline/current.json
--- handoff ---
>>> NEXT
Run pipeline-arch on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
Model: frontier SOTA required — operator assigns the bot; the pipeline can't verify the model.
First: git pull --rebase; no .env in this repo (config lives at ~/.config/oh-my-ib/config.toml, not needed for arch).
Read for context (before acting):
  - oh-my-ib/AGENTS.md — repo conventions (agent-first authoring, hard safety rules, verify model). Read FIRST.
  - .pipeline/pnl-command/PRD.md — what (this feature)
  - src/ib/account.rs — the closest sibling pattern (connect → subscription → take fields → JSON null-helper)
  - src/ib/quote.rs + tests/quote_ticks.rs — the pure-seam freeze pattern (quote_price_tick) to mirror for pnl_number
  - tests/cli_contract.rs — the black-box CLI freeze style to extend for `pnl`
Your task (concrete, numbered):
  1. Lock the design in arch.md: new src/ib/pnl.rs with `pnl(cfg)` + the pure `pnl_number(Option<f64>)->Value`
     seam; cli.rs Command::Pnl (no args); main.rs dispatch; mod.rs `pub use pnl::{pnl, pnl_number}`.
  2. Specify the frozen test surface: tests/pnl_command.rs (offline) = black-box (`--help` lists `pnl`,
     `pnl --help` ok) + pure seam (Some(123.45)->number; Some(1.7976931348623157e308)->null; Some(INF)->null;
     None->null). Gateway wiring is NOT frozen — reviewed-by-reading + live acceptance.
  3. Specify JSON contract: {account, daily_pnl, unrealized_pnl, realized_pnl} (snake_case, mirror account.rs);
     no currency field. Decide whether pnl_number needs ibapi as a dev-dependency (PnL fields are plain f64/
     Option<f64>, so the seam test needs NO ibapi import — unlike quote_ticks which constructs TickTypes).
  4. Note ADR only if a real decision needs recording (likely none new — reuses locked Phase 1 ADRs).
Feature gotchas (project-specific traps the next node MUST know):
  - reqPnL is a CONTINUOUS stream with NO `End` marker (unlike account_updates) — impl takes the FIRST PnL
    then drops the subscription; do NOT loop waiting for End.
  - IB "no value" = Double.MAX_VALUE (1.7976931348623157e308), a real f64, not null — the pnl_number seam
    MUST drop it (and any non-finite) to null. This is the operator-locked behavior (decision A).
  - Binary+lib crate: gateway behavior can't be frozen offline; freeze only the black-box CLI + pure seam.
  - Do NOT touch other frozen specs (phase1 cli_contract, tz, connect-retry, quote, agents-md, slim-claude-md).
  - Live acceptance is BLOCKED until the operator reopens the Tiger gateway on :4001 — offline gates still run.
Done when: arch.md (+ CONTEXT.md if the glossary needs it) committed to trunk; then run pipeline-task.
On failure: attempts++; >=3 ⇒ blocked ⇒ run pipeline-hunt.
<<< END

## seq=2 · 2026-06-30T16:04:55Z · arch→task · completed · by=claude-opus-4-8(claude-code)
done:   Locked design (all PRD claims code-verified). New src/ib/pnl.rs = gateway fn `pnl(cfg)` + pure
        frozen seam `pnl_number(Option<f64>)->Value`, mirroring the account.rs/quote.rs split. Verified:
        connect+resolve_account are pub(crate) & return Client/AccountId; client.pnl(&account,None)->
        Subscription<PnL{daily_pnl:f64, unrealized_pnl/realized_pnl:Option<f64>}>; render_table is GENERIC
        over Value so --format table is free (untouched output.rs); main.run() returns Value. KEY trap →
        ADR 0007: reqPnL is an UNBOUNDED stream with NO End marker, so take ONE reading via
        Subscription::next_data() — a drain-to-End loop (account/quote pattern) would hang. Sentinel
        f64::MAX(1.7976931348623157e308)/non-finite -> null in pnl_number (decision A). Wrote arch.md,
        CONTEXT.md (PnL terms), ADR 0007. Reuses Phase 1 ADRs 0001-0006.
output: .pipeline/pnl-command/arch.md, .pipeline/pnl-command/CONTEXT.md,
        .pipeline/pnl-command/docs/adr/0007-pnl-take-first-unbounded-stream.md, .pipeline/current.json
--- handoff ---
>>> NEXT
Run pipeline-task on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
Model: frontier SOTA required — operator assigns the bot; the pipeline can't verify the model.
First: git pull --rebase; no .env in this repo (config at ~/.config/oh-my-ib/config.toml, not needed).
Read for context (before acting):
  - oh-my-ib/AGENTS.md — repo conventions (agent-first, hard safety rules, verify model). Read FIRST.
  - .pipeline/pnl-command/PRD.md — what
  - .pipeline/pnl-command/arch.md — how (component boundaries, data flow, frozen surface)
  - .pipeline/pnl-command/CONTEXT.md — PnL glossary + the unset-sentinel hazard
  - .pipeline/pnl-command/docs/adr/0007-pnl-take-first-unbounded-stream.md — the no-End take-first decision
  - tests/cli_contract.rs (black-box style to extend) + tests/quote_ticks.rs (pure-seam freeze pattern)
Your task (concrete, numbered):
  1. ONE card (tasks/01.md). Freeze ALL of this feature's red tests in tests/pnl_command.rs (offline):
       a. black-box (assert_cmd, mirror cli_contract.rs): `omi --help` stdout contains "pnl";
          `omi pnl --help` exits 0.
       b. pure seam (NO ibapi import needed — pnl_number takes plain Option<f64>):
          pnl_number(Some(123.45))==json!(123.45); pnl_number(Some(1.7976931348623157e308))==Value::Null;
          pnl_number(Some(f64::INFINITY))==Value::Null; pnl_number(Some(f64::NAN))==Value::Null;
          pnl_number(None)==Value::Null.
     The seam test imports `oh_my_ib::ib::pnl_number` — RED now (module doesn't exist yet → won't compile/fail).
  2. Two-commit freeze (CONTRACT §Test ownership): (1) freeze commit = ONLY tests/pnl_command.rs, must
     compile-and-FAIL → its sha = the feature spec-rev; (2) record commit = write tasks/01.md frontmatter
     + advance current.json.stage=task (metadata only, never spec-paths).
  3. Card 01 frontmatter (exact):
       spec-paths: [tests/pnl_command.rs]
       impl-paths: [src/ib/pnl.rs, src/ib/mod.rs, src/cli.rs, src/main.rs]   (∩ spec-paths = ∅)
       verify:     [cargo build, cargo test --test pnl_command]              (card-scoped, NOT full suite)
     Set current.json.full-verify = [cargo build, cargo test]   (whole-suite gate for review).
  4. Card `## Freeze coverage` section: FROZEN = black-box CLI contract + pure pnl_number seam.
     REVIEW MUST READ (not frozen): client.pnl() wiring, next_data() take-first (ADR 0007),
     resolve_account reuse, JSON assembly {account,daily_pnl,unrealized_pnl,realized_pnl}, --format table.
Feature gotchas (project-specific traps the next node MUST know):
  - reqPnL is a CONTINUOUS stream, NO End marker — impl uses Subscription::next_data() to take ONE reading;
    a for-loop over iter_data() (the account/quote pattern) HANGS. (ADR 0007.)
  - IB "no value" = f64::MAX (1.7976931348623157e308), a real f64 not null — pnl_number drops it (+ any
    non-finite) to null. Operator-locked (decision A).
  - --format table is FREE: render_table is generic over serde_json::Value; do NOT add table code, do NOT
    touch src/output.rs.
  - Do NOT touch other frozen specs: phase1 (cli_contract.rs), tz_aliases.rs, connect_retry.rs,
    quote_ticks.rs, data_commands.rs, agents_md.rs, claude_md.rs. The freeze commit touches ONLY
    tests/pnl_command.rs.
  - Live acceptance is BLOCKED until the operator reopens the Tiger gateway on :4001 — all offline gates
    (build/clippy/test + the frozen red test) still run without it.
Done when: freeze commit + record commit on trunk (main), current.json.stage=task with full-verify set;
  then run pipeline-impl.
On failure: attempts++; >=3 ⇒ blocked ⇒ run pipeline-hunt.
<<< END
