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
