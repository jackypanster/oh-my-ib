# journal — brief-command

## seq=1 · 2026-07-03T02:06:15Z · prd→arch · completed · by=claude-fable-5(claude-code)
done:   New feature: `omi brief` — the daily snapshot (account_summary + pnl + pnl_by_position +
        positions + orders + executions) in ONE gateway connection, one composite JSON. Chosen in a
        /think ROI pass over option-chains (unproven need, entitlement risk) and multi-symbol quote
        (side flow): the daily flow costs 6–7 connects today AND back-to-back reconnects are the
        documented EAGAIN race (src/ib/mod.rs:38-48) — brief removes the class at its cause; all six
        data paths already live-proven on Tiger ⇒ near-zero API risk. Operator locked (HITL): D1
        feature choice; D2 verbatim-nesting shape (account hoisted once, sections = source payloads
        minus wrapper, preview-confirmed); D3 whole-command fail-fast (repo no-partial rule); D4 name
        `brief` (snapshot/summary collide with existing terminology). Code-locked: D5 one connection,
        sequential fetch via *_with_client refactor seams; D6 as_of = gateway server_time. Key risk
        for arch: the full six-dataset one-session interleaving is new as a whole (ADR 0009 proved
        the hardest pair: account_updates drain → N×pnl_single); single account_updates drain should
        feed 3 sections (AccountValue + PortfolioValue in one stream). Decision-complete PRD written;
        current.json repointed to brief-command @ prd.
output: .pipeline/brief-command/PRD.md, .pipeline/current.json, .pipeline/brief-command/journal.md
--- handoff ---
>>> NEXT
Run pipeline-arch on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
Model: frontier SOTA required — operator assigns the bot; the pipeline can't verify the model.
First: git pull --rebase; no .env in this repo (config lives at ~/.config/oh-my-ib/config.toml, not needed for arch).
Read for context (before acting):
  - oh-my-ib/AGENTS.md — repo conventions (agent-first authoring, hard safety rules, verify model). Read FIRST.
  - .pipeline/brief-command/PRD.md — what (D1–D6 locked; success criteria 1–10; risks name your verification targets)
  - .pipeline/pnl-by-position/arch.md + docs/adr/* — prior art: ADR 0007 (take-first, markerless streams), ADR 0009 (two-phase sweep on one session)
  - src/ib/mod.rs — connect/retry layer + the EAGAIN comment (the race brief kills)
  - src/ib/{account,pnl,pnl_by_position,positions,orders,executions}.rs — the six fetch paths brief orchestrates
Your task (concrete, numbered):
  1. grill-with-docs the architecture against the codebase: the *_with_client refactor seam for each
     of the six modules (public fn keeps its own connect; brief shares one Client).
  2. Verify in ibapi-3.1.0 SOURCE (not guessed) per-pair session safety of the sequential interleaving:
     account_updates drain → pnl take-first → N×pnl_single take-first → open_orders → executions
     (request-id isolation, subscription cleanup on drop, singleton subscriptions like reqAccountUpdates).
     Decide the fetch order + whether ONE account_updates drain feeds account_summary/positions/
     pnl_by_position-discovery (AccountValue + PortfolioValue in one pass) — PRD Scope expects yes.
  3. Emit arch.md + CONTEXT.md + docs/adr/* (ADR for the one-session interleaving + the shared-drain
     decision; record the fallback deform — internal sequential sessions, distinct client_ids — as
     last resort). Advance current.json.stage=arch, append journal seq=2, commit once, push.
Feature gotchas (project-specific traps the next node MUST know):
  - Fail-fast no-partial is a repo IRON RULE (pnl_by_position.rs header) — no per-section error objects.
  - Section shapes are FROZEN BY REFERENCE to the six source commands (PRD criterion 2 hoisting rule);
    brief adds NO new row shapes — arch must not invent any.
  - orders.rs emits {"open_orders":[...]} with NO account wrapper (unlike the other five) — the
    hoisting rule already accounts for it; don't "fix" it.
  - ADR 0007: pnl/pnl_single streams are markerless — take-first, NEVER drain-to-End; a drain loop
    hangs forever. account_updates DOES have an End marker — drain it.
  - Public repo: no account ids/balances in any committed artifact.
Done when: arch.md + CONTEXT.md + ADRs on trunk, journal seq=2 appended, stage=arch pushed.
On success: run pipeline-task. On failure: attempts++; >=3 ⇒ blocked ⇒ run pipeline-hunt.
<<< END
