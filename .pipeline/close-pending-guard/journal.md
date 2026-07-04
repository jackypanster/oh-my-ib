# Run journal — close-pending-guard

## seq=1 · 2026-07-04T08:58:11Z · prd→arch · completed · by=claude-fable-5
done:   PRD decision-complete: pending-close guard in option_close (brute-force refuse when a
        working opposite-side order exists on the conid; not_found naming ids; reuse shared
        open_orders_with_client drain on the same client). Fixes audit finding #1 (double-fire
        flip). ONE card. Deferred paper lifecycle combined with option-close criterion 12.
output: .pipeline/close-pending-guard/PRD.md
--- handoff ---
>>> NEXT
Run pipeline-arch on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
Model: frontier SOTA required — operator assigns the bot; the pipeline can't verify the model.
First: git pull --rebase; no repo .env.
Read for context (before acting):
  - AGENTS.md — repo conventions + hard safety rules
  - .pipeline/close-pending-guard/PRD.md — what + D1-D5
  - src/ib/trade.rs (option_close fn) — the insertion point (after non-OPT check, before derive_close)
  - src/ib/orders.rs — open_orders_with_client shared drain (reuse VERBATIM, pub(crate))
  - .pipeline/option-close/docs/adr/0022 — the gates this feature completes
Your task (concrete, numbered):
  1. grill-with-docs: (a) exact seam signature blocking_close_order_ids(position: f64, conid: i32,
     orders: &[(i32, i32, String|&str)]) -> Vec<i32> and triple-extraction from the drain's JSON
     rows (or decide raw-OrderData extraction — pick ONE, justify); (b) guard wiring order inside
     option_close; (c) refusal message exact content; (d) AGENTS.md phrase.
  2. Emit arch.md + CONTEXT.md + ADR 0023 (guard semantics: fail-closed, no status filter,
     no account filter, TOCTOU residual accepted).
  3. Journal seq=2 rides the commit; push.
Feature gotchas (project-specific traps the next node MUST know):
  - derive_close and shape_option_close_ack are FROZEN (tests/option_close_command.rs) — the guard
    must refuse BEFORE derive_close, signatures untouched.
  - Single-connect invariant: the guard drain uses the SAME client already connected in option_close.
  - all_open_orders returns only working orders (terminal ⇒ completed-orders) — no status filter.
  - CLAUDE.md must NOT change (byte budget 876/900 — do not spend it).
Done when: arch.md + CONTEXT.md + ADR 0023 on trunk (journal seq=2 same commit).
On success: run pipeline-task. On failure: attempts++; >=3 ⇒ blocked ⇒ run pipeline-hunt.
<<< END
