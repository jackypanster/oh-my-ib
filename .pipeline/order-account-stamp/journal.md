# Run journal — order-account-stamp

## seq=1 · 2026-07-04T09:26:59Z · prd→arch · completed · by=claude-fable-5
done:   PRD decision-complete: stamp resolved account onto Order at the place_with_client
        choke point (D1); overwrite semantics (D2); frozen builders untouched (D3 — verified
        no frozen test asserts order.account); option_close reuses its resolved account (D4).
        Fixes audit finding #2 (cross-account anti-open-gate defeat). ONE card. Paper
        acceptance runnable TODAY (acks, no fills).
output: .pipeline/order-account-stamp/PRD.md
--- handoff ---
>>> NEXT
Run pipeline-arch on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
Model: frontier SOTA required — operator assigns the bot; the pipeline can't verify the model.
First: git pull --rebase; no repo .env.
Read for context (before acting):
  - AGENTS.md — repo conventions + hard safety rules
  - .pipeline/order-account-stamp/PRD.md — what + D1-D4
  - src/ib/trade.rs — place_with_client (~:308), place_core (~:365), option_combo (~:605),
    option_close (~:765 resolve_account already present) — the wiring surface
  - src/ib/mod.rs resolve_account (:99) — AccountId; .0 access precedent in positions.rs
Your task (concrete, numbered):
  1. grill-with-docs: pin the place_with_client signature change (account param: &AccountId
    vs resolve-inside-with-cfg — PRD D4 forbids a duplicate managed_accounts on the
    option_close path; pick and justify), the seam signature, and the AGENTS.md phrase.
  2. Emit arch.md + CONTEXT.md + ADR 0024 (choke-point stamping, overwrite semantics,
     Tiger-accepts-explicit-account assumption + fallback).
  3. Journal seq=2 rides the commit; push.
Feature gotchas (project-specific traps the next node MUST know):
  - Frozen suites (stk_orders/option_orders/option_combo/option_close/positions_row/
    close_pending_guard) must remain byte-untouched; builders' signatures FROZEN.
  - CLAUDE.md must NOT change (876/900 budget).
  - Write/Edit metadata via tools with loud assertions — never silent replace/heredocs
    (2026-07-04 lesson, twice).
Done when: arch.md + CONTEXT.md + ADR 0024 on trunk (journal seq=2 same commit).
On success: run pipeline-task. On failure: attempts++; >=3 ⇒ blocked ⇒ run pipeline-hunt.
<<< END
