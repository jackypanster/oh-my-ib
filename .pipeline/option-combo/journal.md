
## seq=3 · 2026-07-04T02:28:30Z · task→impl · completed · by=claude-fable-5
done:   spec FROZEN: freeze commit c0e72a3 (= spec-rev) carries card 01's red tests
        (tests/option_combo_command.rs, 21 tests; RED on unresolved oh_my_ib imports only;
        verbatim-compile pre-verified via stub module, deleted pre-commit). ONE card at
        todo/0. Same deliberate gate-pass omission as option-orders (live-order hazard).
output: .pipeline/option-combo/tasks/01.md, tests/option_combo_command.rs (spec-rev c0e72a3)
--- handoff ---
>>> NEXT
pipeline-impl on the omp pane (π), full-auto. repo=git@github.com:jackypanster/oh-my-ib.git
branch=main. Card .pipeline/option-combo/tasks/01.md; arch.md §Component design VERBATIM;
branch feat/option-combo from trunk HEAD; verify cargo test --test option_combo_command +
regression stk_orders_command + option_orders_command + FULL suite + clippy; PR; card
review; journal seq=4. Never touch tests/**; CLAUDE.md < 900B check before PR.
<<< END

## seq=4 · 2026-07-04 · impl(card 01)→review · completed · by=glm-5.2 (omp)
done:   card 01 (multi-leg BAG) implemented on feat/option-combo (PR #18): src/ib/trade.rs
        (LegSpec + parse_combo_leg 6-token DSL pure FROZEN seam + build_combo_order via
        SpreadBuilder.add_leg.ratio.done...build()? with symbol back-fill, sign-free net LMT +
        shape_combo_order_ack 7+7-key pure FROZEN seam + option_combo gateway fn with validation
        [action BUY|SELL ci, legs 2..=4, per-leg parse naming "leg N: reason", same-symbol rule,
        qty whole>=1, limit finite ANY SIGN] < gate < connect, per-leg contract_details conid
        resolve fail-fast, place_core reuse). cli OptionCombo + OptionComboArgs (--leg singular
        via long="leg"; allow_hyphen_values on --limit for negative credits). Docs two-text rule:
        AGENTS.md full form, CLAUDE.md short form 861B < 900. cargo test --test option_combo_command
        21/21 green; stk 16/16 + option-orders 21/21 green (place_core untouched); FULL cargo test
        181/181 green (23 suites); clippy --all-targets -D warnings clean; freeze gates c0e72a3..tip
        -- tests/{option_combo,stk_orders,option_orders}_command.rs ALL empty. PR #18 open.
output: src/ib/trade.rs, PR https://github.com/jackypanster/oh-my-ib/pull/18
--- handoff ---
>>> NEXT
Run pipeline-review on a FRESH session (rebuild from repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=feat/option-combo pr=https://github.com/jackypanster/oh-my-ib/pull/18
Model: frontier SOTA required — review is a reasoning stage; operator assigns the bot.
First: git fetch origin; git checkout feat/option-combo (tip 6d470a1 — card 01 impl, spec-rev c0e72a3).
Read for context (before acting):
  - .pipeline/option-combo/tasks/01.md — card 01 at review; spec-rev c0e72a3
  - tests/option_combo_command.rs — the frozen spec (21 tests; DO NOT diff-edit)
  - tests/stk_orders_command.rs + tests/option_orders_command.rs — regression specs (place_core)
  - tests/claude_md.rs + tests/agents_md.rs — frozen doc tests (byte budget + markers)
Your task (CONTRACT §Test ownership + §State authority):
  1. Freeze gate FIRST: `git diff c0e72a3 <review-tip> -- tests/` non-empty ⇒ reject.
     Expected: empty across ALL tests/ (combo + stk + option-orders + claude_md + agents_md).
  2. Semantic review (WRITE polarity — highest scrutiny):
     a. place_core reuse: byte-identical to option-orders — stk + option-orders frozen suites green.
     b. Containment grep: write symbols (place_order/cancel_order) ONLY in src/ib/trade.rs.
     c. build_combo_order: SpreadBuilder usage (add_leg per spec, ratio, done, build()?), symbol
        back-fill, sign-free net limit (negative = credit — deliberately unlike single-leg).
     d. parse_combo_leg: 6-token DSL normalization, Err reasons wrapped as "leg N: reason" usage.
     e. option_combo gateway: validation ordering (usage < gate < connect); per-leg fail-fast
        conid resolve via contract_details FIRST row; leg-index naming.
     f. CLI: --leg singular (long="leg"); --limit allow_hyphen_values (negative credits); Vec<String>.
     g. Docs two-text rule: AGENTS.md full form (verbatim), CLAUDE.md short form (< 900B).
  3. Full-suite gate: `cargo build && cargo test` — ALL GREEN (181 tests). clippy clean.
  4. Human confirm → squash-merge PR #18 (the only merge). Card → done.
  5. Live acceptance (operator, paper :4002, PRD criteria 1-2, MERGE GATE): far-net-debit
     vertical → orders shows BAG working → cancel → completed-orders Cancelled → positions
     unchanged. SMART BAG quirks journaled as observations.
Feature gotchas:
  - Sign-free net limit: negative = credit. This is DELIBERATELY unlike single-leg (where limit>0
    is required). The frozen test asserts Some(-0.5) builds successfully.
  - --leg uses long="leg" (singular) — clap would otherwise generate --legs from the field name.
  - SpreadBuilder.build() returns Result<Contract, Error> (can fail if 0 legs) — build_combo_order
    returns Result<(Contract, Order), String>.
  - CLAUDE.md = 861 bytes (under 900 frozen budget by 39).
  - No arch.md/ADR/CONTEXT/PRD committed for this feature (pipeline compressed them into the card);
    the card §Scope + frozen test ARE the contract.
Done when: freeze gate empty, semantic review clean (esp. place_core byte-identity + containment +
sign-free limit + SpreadBuilder), full-suite green, human confirms, PR #18 squash-merged, card→done.
On failure: flip card review→todo + attempts++; attempts>=3 ⇒ blocked ⇒ pipeline-hunt.
<<< END
