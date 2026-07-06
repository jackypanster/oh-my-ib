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

## seq=2 · 2026-07-06T09:33:03Z · arch→task · completed · by=claude-opus-4-8
done:   Arch + ADR 0030 accepted (all D1-D5 confirmed with operator incl D5 exempt=close). Shape: 4 PURE
        seams in trade.rs + thin wiring. Frozen seams: compute_notional(qty,limit:Option,mult)->Option
        (mirror shape_preview); resolve_max_notional(Option<&str>)->Result (None⇒500.0 DEFAULT; parse,
        finite∧>0 else Err — fail-closed); check_live_write_posture(is_live,is_mkt,notional,cap)->Result
        (!live⇒Ok; mkt⇒Err LMT; >cap⇒Err naming notional/cap/OMI_MAX_NOTIONAL; else Ok);
        refuse_live_combo_on_live(is_live)->Result. const DEFAULT_MAX_NOTIONAL=500.0. All refuses=
        AppError::config (exit5). Wiring (review-by-reading): place_core (468) posture check AFTER gate,
        BEFORE connect (multiplier from contract.security_type; is_mkt from order.order_type; env read
        inline like the gate); option_combo (766) refuse_live_combo before the gate on the real path.
        UNTOUCHED: require_live_write_gate body, place_with_client (⇒ option-close exempt structurally),
        option_close, cancel, shape_preview JSON (order-preview tests must stay green), config.rs (env
        primary, no toml key this card). Freezable (refuses offline-deterministic) — NEW spec file, no
        re-freeze. Live-pass path = operator live acceptance (first trial order).
output: .pipeline/live-write-guardrail/{arch.md,CONTEXT.md,docs/adr/0030-live-write-guardrail.md}
--- handoff ---
>>> NEXT
Run pipeline-task on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
Model: frontier SOTA required — operator assigns the bot; the pipeline can't verify the model.
First: git pull --rebase.
Read: .pipeline/live-write-guardrail/{PRD.md,arch.md,CONTEXT.md,docs/adr/0030-*}; src/ib/trade.rs
      (shape_preview 79/85 the notional math; place_core 468; place_with_client 349; option_combo 713/766;
      build_stk_order 31/44 MKT signal); src/ib/mod.rs:45 (re-export line); src/config.rs LIVE_PORT 14;
      tests/order_preview_command.rs (shape_preview JSON must stay green); an existing pure-seam test file
      for style (e.g. tests/option_chain_filter.rs).
Your task (concrete, numbered):
  1. Write ONE red spec file tests/live_write_guardrail.rs importing
     oh_my_ib::ib::{compute_notional, resolve_max_notional, check_live_write_posture,
     refuse_live_combo_on_live}. It MUST compile-fail NOW (unresolved imports — the seams don't exist yet)
     — that is the genuine RED. Assertions (~appropriate, not 100%):
       - compute_notional: LMT value uses |limit| (e.g. 2×|3.0|×100=600; STK mult 1); MKT (None)⇒None.
       - resolve_max_notional: None⇒500.0; Some("1000")⇒1000.0; Some("abc")/Some("0")/Some("-5")/
         Some("")⇒Err (fail-closed). (Some("inf") — decide + assert per ADR: finite required ⇒ Err.)
       - check_live_write_posture: paper (is_live=false) ⇒ Ok even for is_mkt=true / huge notional; live
         MKT (is_mkt=true) ⇒ Err; live over-cap (Some(600),cap 500) ⇒ Err; live within-cap (Some(300),cap
         500) ⇒ Ok; boundary Some(500)==cap ⇒ Ok (> is the refuse, not >=).
       - refuse_live_combo_on_live: true⇒Err, false⇒Ok.
     Card 01 is a SINGLE card (one observable behaviour: "the live write posture guardrail").
  2. Freeze in TWO commits (CONTRACT §spec-rev double-commit): (a) freeze commit = ONLY
     tests/live_write_guardrail.rs → its sha = spec-rev; (b) record commit = tasks/01.md frontmatter
     (status: todo, attempts: 0, verify: [cargo build, cargo test --test live_write_guardrail],
     spec-paths: [tests/live_write_guardrail.rs], impl-paths: [src/ib/trade.rs, src/ib/mod.rs, src/cli.rs],
     spec-rev: <sha from a>) + current.json stage=task (full-verify [cargo build, cargo test]) + journal
     seq=3. NEVER mix the test and the card in one commit.
  3. In the card's `## Freeze coverage`: FROZEN = the 4 pure seams (above); REVIEW-BY-READING = the
     place_core/option_combo wiring (per ADR 0030 §Seams); OPERATOR LIVE ACCEPTANCE = the refuse commands
     exit 5 / the within-cap first trial order.
Feature gotchas:
  - spec-paths ∩ impl-paths = ∅ (test file vs src). The seams are NEW pub fns in trade.rs re-exported at
    mod.rs:45 — the coder adds them; the red is the unresolved import.
  - shape_preview JSON output must stay byte-identical (order-preview frozen tests) whether or not the
    coder routes it through compute_notional. Flag this in the card.
  - option-close / cancel / preview / paper MUST stay unchanged — the check lives in place_core (NOT
    place_with_client) precisely so closes stay exempt.
  - Do NOT freeze the within-cap→place path (would place a real live order) — that is operator acceptance.
  - Safe to develop with the Tiger gateway UP: every guardrail refuse is offline (no connect); the frozen
    tests are pure (no gateway).
Done when: tests/live_write_guardrail.rs frozen (compiles+FAILS on unresolved imports); card 01 recorded;
current.json stage=task; journal seq=3 pushed. On success: task→impl, run pipeline-impl (assign π/OMP).
On failure: attempts++; >=3 ⇒ blocked ⇒ run pipeline-hunt.
<<< END
