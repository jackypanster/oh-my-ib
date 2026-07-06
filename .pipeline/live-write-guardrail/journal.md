# journal — live-write-guardrail

## seq=1 · 2026-07-06T09:26:15Z · prd→arch · completed · by=claude-opus-4-8
done:   PRD written. `omi`'s only write guard is the paper/live port gate; it says nothing about whether
        the live order is sane (magnitude/side/price), and the order is composed by CC from natural
        language with no deterministic breaker (2026-07-06 incident = a wrong instruction reached a real
        order). Fractional is API-dead ([10243], verified on paper today) ⇒ min live fill = 1 whole share
        ⇒ a notional cap is the only economic breaker. Goal: on LIVE opening orders, refuse (offline,
        before connect, in trade.rs) anything not-LMT / over-cap / combo; paper untouched. Decisions
        locked with operator (3 questions): D1 live must be LMT (MKT refused; STK-only bite); D2 notional
        = qty×|limit|×mult (STK×1/OPT×100), >cap ⇒ refuse (reuse shape_preview math); D3 cap default $500,
        OMI_MAX_NOTIONAL overrides (bad value ⇒ fail-closed); D4 live combo refused (interlock: combo
        paper-only); D5 cap on OPENING orders only (buy/sell/option-buy/option-sell via place_core) —
        option-close EXEMPT (never block an exit; bypasses place_core), cancel N/A, preview exempt.
        Freezable (refuse paths are offline-deterministic) — NEW spec file, no re-freeze. Live-trial
        BLOCKER. Trade log = the NEXT feature (operator decided), not this card.
output: .pipeline/live-write-guardrail/PRD.md
--- handoff ---
>>> NEXT
Run pipeline-arch on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
Model: frontier SOTA required — operator assigns the bot; the pipeline can't verify the model.
First: git pull --rebase; load repo config (.env if present, per CONTRACT step 2).
Read for context (before acting):
  - oh-my-ib/AGENTS.md + CLAUDE.md — repo conventions (agent-first; hard safety rules; Verify = 4 gates)
  - .pipeline/live-write-guardrail/PRD.md — problem + the 5 locked decisions (D1-D5) + rejected alts
  - src/ib/trade.rs — the write module. KEY seams: require_live_write_gate (175, the port gate — extend
    ALONGSIDE, never weaken); shape_preview (79, has the notional math qty×|limit|×mult to extract);
    place_core (468, shared by buy/sell/option-buy/option-sell — the wire point for the posture check);
    place_with_client (349, the choke point — option-close routes here directly, so it is EXEMPT by
    construction; do NOT add the check here); option_combo (713, add a live-refuse line at its gate site
    766); build_stk_order (31, limit None ⇒ order_type MKT — the D1 signal).
  - src/config.rs — LIVE_PORT (14), Config (port/preview), merge_flags; decide if OMI_MAX_NOTIONAL is
    read in config.rs or in trade.rs (the gate reads env directly in trade.rs — precedent).
  - src/error.rs — AppError kinds (config=exit5 is the refuse code; usage=64 an option for the MKT case).
Your task (concrete, numbered):
  1. Confirm the mechanism + emit ADR 0030 (next number). Pin: (a) compute_notional pure seam extracted
     from shape_preview (STK×1 / OPT×100; MKT ⇒ None ⇒ no cap number but D1 refuses live MKT first);
     (b) a pure live-posture decision fn — inputs (is_effective_live, is_mkt, notional: Option<f64>,
     cap) → Ok / Err(reason); (c) the OMI_MAX_NOTIONAL reader (absent⇒$500 default const; present⇒parse,
     ≤0 or non-numeric ⇒ config error — fail-closed); (d) the wire: posture check inside place_core
     AFTER the preview branch and paired with require_live_write_gate (both offline, before connect) +
     a live-refuse in option_combo; option-close/cancel/paper/preview untouched.
  2. Decide the exact error kind per refuse (recommend: config for over-cap + combo + bad-env; usage OR
     config for live-MKT — pick one and freeze it). Decide error message shape (must name notional, cap,
     and the OMI_MAX_NOTIONAL escape for the cap case).
  3. Decide cap default location: a `const DEFAULT_MAX_NOTIONAL: f64 = 500.0` (where?) + whether a
     config.toml key is added now (PRD says env primary, config optional — recommend env-only this card).
  4. Write arch.md (chosen shape + component boundaries: which fns are pure/frozen vs gateway-wired) +
     CONTEXT.md (glossary: the port gate vs the write POSTURE guard; opening vs closing order; notional;
     effective-live) + docs/adr/0030-*.md. Do NOT write src or tests.
Feature gotchas (project-specific traps the next node MUST know):
  - Guard fires ONLY when cfg.port == LIVE_PORT AND !cfg.preview. Paper + preview EXEMPT by construction.
  - option-close is EXEMPT (never block an exit) — it bypasses place_core; do NOT put the check in
    place_with_client or you will catch closes. cancel = N/A (no notional).
  - D1 (live must be LMT) only bites STK — options/combo are already LMT-only. The cap (D2) applies to all
    opening verbs.
  - Fail-closed on a bad OMI_MAX_NOTIONAL — a typo must REFUSE, never fall back to $500.
  - The within-cap→place path is NOT freezable (would place a real live order) — that is operator live
    acceptance (the first trial order). Freeze the REFUSE decisions + notional math + env parse only.
  - Do NOT weaken require_live_write_gate; extend alongside it.
Done when: arch.md + CONTEXT.md + ADR 0030 exist; mechanism + error kinds + cap-default decided;
current.json stage=arch; journal seq=2 appended + pushed. On success: arch→task, run pipeline-task.
On failure: attempts++; >=3 ⇒ blocked ⇒ run pipeline-hunt.
<<< END
