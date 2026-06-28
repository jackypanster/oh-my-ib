# journal — phase1-readonly

## seq=1 · 2026-06-28T04:41:04Z · prd→arch · completed · by=claude-opus-4-8(claude-code)
done:   Onboarded oh-my-ib to the pipeline (roles.yaml, CLAUDE.md, .gitignore) and wrote the
        decision-complete PRD for the read-only IBKR CLI. Grilling was already done in the
        originating /think session, so all decisions (D1–D7) are locked in PRD.md.
output: .pipeline/phase1-readonly/PRD.md, .pipeline/current.json, .pipeline/roles.yaml, CLAUDE.md
--- handoff ---
>>> NEXT
Run pipeline-arch on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
First: git pull --rebase; load repo config (.env if present, per CONTRACT step 2).
Read for context (before acting):
  - oh-my-ib/CLAUDE.md            — project conventions + hard safety rules (read FIRST)
  - .pipeline/phase1-readonly/PRD.md  — what: read-only IBKR CLI `omi`, locked decisions D1–D7
  - .pipeline/roles.yaml         — arch slot = grill-with-docs
Your task (concrete, numbered):
  1. Survey: this is an EMPTY repo (no Cargo project yet) — arch must define the initial crate layout.
  2. Produce arch.md: module boundaries for a sync `ibapi` CLI — connection helper (connect/disconnect),
     clap command tree (health/account/positions/orders/quote/contract/history), output layer
     (json|table), config loader (~/.config/oh-my-ib/config.toml + flag overrides), error type
     (anyhow + structured {"error":{code,message,context}} on stderr, non-zero exit).
  3. Produce CONTEXT.md: domain glossary (IB Gateway, TWS API, paper vs live ports 4002/4001,
     conId, market-data type delayed/live/frozen, NetLiquidation/BuyingPower, client_id).
  4. Write ADRs under docs/adr/ for the locked decisions: D1 TWS-API-via-ibapi, D2 sync-client,
     D3 stateless-connect-per-command, D4 json-first-output, D5 paper-default/live-opt-in.
  5. Confirm the `ibapi` 3.1 sync API surface against docs.rs before locking module signatures
     (account_summary, positions, open_orders, market data snapshot, contract_details, historical_data).
Feature gotchas (project-specific traps the next node MUST know):
  - Repo is PUBLIC: arch must keep all account ids/secrets in ~/.config (gitignored), never in-repo.
  - Phase 1 is structurally read-only: do NOT design any order-placement module.
  - Binary crate, binary name `omi`, package `oh-my-ib`.
  - ibapi sync client only: `ibapi = { version = "3.1", default-features = false, features = ["sync"] }`.
  - Default market data = delayed (most users lack a realtime subscription).
Done when: arch.md + CONTEXT.md + ADRs committed to trunk; current.json.stage=arch. On success:
  append a journal entry, then run pipeline-task.
On failure: attempts++; >=3 ⇒ blocked ⇒ run pipeline-hunt.
<<< END
