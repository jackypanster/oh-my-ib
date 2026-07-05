# Journal — preview-readonly

## seq=1 · 2026-07-05T15:34:26Z · prd→arch · completed · by=claude (cc)
done:   PRD for the R1-refuted fix. Live-acceptance of order-preview (PR #23) on the LIVE Tiger
        gateway (account U20230856) proved Tiger TRANSMITS whatIf orders (a real resting order
        appeared; price-band-rejected; zero financial impact). So `--preview` must be re-implemented
        READ-ONLY: resolve the contract via client.contract_details (no place_order/what_if), echo the
        order params, compute notional. 2 HITL decisions confirmed by operator: (1) gate = read-shaped
        (--live only, no OMI_ALLOW_LIVE — it's a pure read now); (2) full envelope (contract + echo +
        notional + transmits:false; drop margin/commission/what_if). Module placement (trade.rs vs new
        preview.rs) deferred to arch. Operator directive: for now ONLY cc+omi (Hermes/TG deferred until
        the CLI is stable); cc runs the live-acceptance directly.
output: .pipeline/preview-readonly/PRD.md, .pipeline/current.json (stage=prd)
--- handoff ---
>>> NEXT
Run pipeline-arch on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=feat/preview-readonly pr=none
Model: frontier SOTA required.
First: git pull --rebase; load repo config (.env if present, per CONTRACT step 2).
Read for context (before acting):
  - oh-my-ib/AGENTS.md + CLAUDE.md — repo conventions. NOTE: "write code ONLY in src/ib/trade.rs" targets place_order/cancel_order (writes); the read-only preview has NO writes, so it MAY live elsewhere (see Decision 6).
  - .pipeline/preview-readonly/PRD.md — what: re-implement --preview READ-ONLY (contract_details, no place_order), read-shaped gate, full envelope with notional + transmits:false, all 6 verbs.
  - .pipeline/order-preview/ (PRD/arch/CONTEXT/ADR 0026) + docs/write-path-semantics.md — the shipped preview being fixed; CONTEXT.md R1 is the refuted premise.
Your task (concrete, numbered):
  1. Read src/ib/trade.rs: preview_with_client (:423, sets order.what_if=true :432 then place_order — the path to REMOVE), shape_preview (:77), the three branch sites (place_core:481, option_combo:820, option_close:1072), the builders (build_stk_order/build_option_order/build_combo_order), require_live_write_gate (:143).
  2. Read src/ib/contract.rs (client.contract_details → conid/exchange/currency/long_name) + src/ib/option_quote.rs (option contract resolution: expiry/strike/right → conid, multiplier) — the read-only resolution to reuse.
  3. DECIDE module placement (PRD Decision 6): keep the read-only preview in trade.rs (reuse builders in place) OR extract to src/ib/preview.rs importing the pure builders. Record as an ADR.
  4. Design the read-only preview fn (replaces preview_with_client): build (Contract, Order) via the builders → client.contract_details(&contract) (READ) → shape the full envelope (contract resolved fields + order echo + notional = qty×limit×multiplier + transmits:false). NO place_order, NO what_if. For options, resolve via the option path + multiplier 100.
  5. Design the read-shaped gate: preview must NOT call require_live_write_gate (or must bypass it) — reachable with --live alone; on dead port ⇒ connection error, not config. Real order path keeps require_live_write_gate.
  6. Confirm containment: place_order/cancel_order appear ONLY on the real order path after the change. The freeze note (PRD): tests/order_preview_command.rs asserts the OLD behavior (what_if:true, gate=config) — it MUST be re-frozen by task for the NEW behavior (transmits:false + notional; gate=connection). Name this in the handoff to task.
  7. Emit arch.md + CONTEXT.md + ADR (module placement + read-shaped gate + envelope). Advance current.json.stage=arch. Append journal seq=2. Do NOT touch src/ or tests/.
Feature gotchas:
  - R1 is REFUTED (Tiger transmits whatIf) — the ENTIRE point is NO place_order in preview. Any design that calls place_order (even what_if) is WRONG.
  - contract_details is a read (ungated for the write-gate); the read-shaped gate is the whole reason preview can drop OMI_ALLOW_LIVE.
  - The old frozen test tests/order_preview_command.rs is the spec-to-CHANGE (not preserve): what_if:true and gate=config must flip. Task re-freezes.
  - Live-acceptance is cc-run (omi directly); Hermes/TG deferred.
Done when: arch.md + CONTEXT.md exist, decision-complete; current.json.stage=arch; journal seq=2. On success: run pipeline-task.
On failure: attempts++; >=3 ⇒ blocked ⇒ run pipeline-hunt.
<<< END
