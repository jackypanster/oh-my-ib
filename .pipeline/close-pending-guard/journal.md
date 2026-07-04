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

## seq=2 · 2026-07-04T09:00:04Z · arch→task · completed · by=claude-fable-5
done:   arch.md + CONTEXT.md + ADR 0023. Seam pinned: blocking_close_order_ids(position, conid,
        &[(order_id, conid, action)]) -> Vec<i32> ascending; guard wired between non-OPT check and
        derive_close via open_orders_with_client(client, None, ctx) — zero new connects/gateway
        calls; malformed drain row ⇒ data error (never skip). CLAUDE.md untouched by design.
        (Also corrected stale current.json.stage left from option-close closure — journal tail
        was authoritative throughout.)
output: .pipeline/close-pending-guard/arch.md, CONTEXT.md, docs/adr/0023-pending-close-guard.md
--- handoff ---
>>> NEXT
Run pipeline-task on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
Model: frontier SOTA required — operator assigns the bot; the pipeline can't verify the model.
First: git pull --rebase; no repo .env.
Read for context (before acting):
  - .pipeline/close-pending-guard/PRD.md + arch.md (§Freeze plan) + docs/adr/0023
  - tests/option_close_command.rs — freeze style precedent (do NOT touch it)
Your task (concrete, numbered):
  1. ONE card: tests/close_pending_guard.rs = the seam matrix from arch.md §Pure FROZEN seam
     (~7 asserts: long+Sell blocks / long+Buy no / short+Buy blocks / short+Sell no /
     other-conid no / multi-blocker ascending / zero-position empty). Pure fn only — no
     binary invocation, no gate tests (frozen elsewhere).
  2. Freeze commit (spec file only, verbatim-compile pre-check via stub, RED on unresolved
     import of oh_my_ib::ib::blocking_close_order_ids only) then record commit (card 01 +
     current.json stage=task + full-verify [cargo build, cargo test] + journal seq=3); push both.
  3. Card verify: [cargo build, cargo test --test close_pending_guard].
     impl-paths: src/ib/trade.rs, src/ib/mod.rs, AGENTS.md. spec ∩ impl = ∅.
Feature gotchas:
  - Existing frozen suites (option_close_command.rs, positions_row.rs) must remain byte-untouched.
  - Seam action strings are "Buy"/"Sell" (Debug format, orders.rs:38 precedent) — freeze exactly.
  - CLAUDE.md must NOT change.
Done when: freeze commit (= spec-rev) + record commit pushed, journal seq=3.
On success: run pipeline-impl (π on omp pane). On failure: attempts++; >=3 ⇒ blocked ⇒ pipeline-hunt.
<<< END
