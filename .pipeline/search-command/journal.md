# journal — search-command (append-only)

## seq=1 · 2026-07-03T08:57:59Z · prd→arch · completed · by=claude-fable-5
done:   PRD decision-complete: omi search <pattern> via reqMatchingSymbols; operator locked
        D1 (feature choice over account-summary-tags/completed-orders), D2 (name=search),
        D3 (full pass-through rows, no STK filter — metadata not market-data). ibapi verified:
        matching_symbols returns Vec<ContractDescription> directly (contracts/sync.rs:143-155),
        no subscription lifecycle — simplest read class in the repo.
output: .pipeline/search-command/PRD.md
--- handoff ---
>>> NEXT
Run pipeline-arch on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
Model: frontier SOTA required — operator assigns the bot; the pipeline can't verify the model.
First: git pull --rebase; no .env in this repo.
Read for context (before acting):
  - AGENTS.md + CLAUDE.md — repo conventions
  - .pipeline/search-command/PRD.md — what + locked decisions D1-D5
  - ~/.cargo/registry/src/*/ibapi-3.1.0/src/contracts/sync.rs:143-155 (matching_symbols) and
    the decode_contract_descriptions decoder + ContractDescription/Contract structs — pin the
    exact emitted row key set from source
  - src/ib/contract.rs — nearest sibling (contract_details request-response shaping) for
    house patterns; src/ib/pnl_by_position.rs shape_* seam pattern for the pure row builder
Your task (concrete, numbered):
  1. Pin the row shape from ibapi decoder source: which ContractDescription/Contract fields
     arrive on SymbolSamples (mind server-version-dependent fields) -> exact JSON key set +
     absent-value rules (house sentinel conventions).
  2. Write arch.md: src/ib/search.rs component (pure shape seam + gateway fn), cli.rs variant,
     mod.rs/main.rs wiring, error mapping (Err -> data envelope, context "search").
  3. Write ADR 0014 (repo-global numbering: 0013 taken) — plain bounded call class (neither
     take-first nor drain), rate-limit note, pass-through decision.
  4. CONTEXT.md — glossary deltas.
  5. Pin freeze coverage: frozen = pure shape seam + CLI contract (help/usage/dead-port);
     review-by-reading = the one-call gateway fn; live = PRD criterion 8.
Feature gotchas (project-specific traps the next node MUST know):
  - NEVER run repo-wide cargo fmt (rewrites frozen tests/) — fmt src/** only.
  - Public repo: no account ids/tokens/balances anywhere.
  - No STK guard in search (PRD D3 — metadata, not market-data); do NOT copy quote's guard.
  - NOT ADR 0012's take-first class — no TAKE_FIRST_TIMEOUT wrapping.
  - ADR numbering is repo-global — next is 0014.
Done when: arch.md + CONTEXT.md + docs/adr/0014-*.md committed (stage=arch, journal seq=2,
one commit, pushed). On success: run pipeline-task.
On failure: attempts++; >=3 => blocked => run pipeline-hunt.
<<< END

## seq=2 · 2026-07-03T09:00:37Z · arch→task · completed · by=claude-fable-5
done:   arch complete: 4-file diff (cli Search variant, NEW src/ib/search.rs with SearchRow +
        shape_search + search, mod.rs re-export, main.rs dispatch); ADR 0014 (plain bounded
        call class — third read class; full pass-through; no STK guard); row shape pinned from
        proto decoder source (7 keys, decoder defaults make shape total). Freeze coverage pinned.
output: .pipeline/search-command/arch.md, .pipeline/search-command/CONTEXT.md,
        .pipeline/search-command/docs/adr/0014-search-plain-bounded-call.md
--- handoff ---
>>> NEXT
Run pipeline-task on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
Model: frontier SOTA required — operator assigns the bot.
First: git pull --rebase; no .env in this repo.
Read for context (before acting):
  - AGENTS.md + CLAUDE.md; .pipeline/search-command/{PRD.md,arch.md,docs/adr/0014-*.md}
  - tests/multi_quote.rs + tests/pnl_by_position_command.rs — house freeze patterns
    (house-red unresolved import; frozen test constructs plain rows directly)
Your task (concrete, numbered):
  1. FREEZE COMMIT: write tests/search_command.rs per arch.md §Freeze coverage — shape_search
     (exact 7-key row; gateway order; "" description pass-through; empty derivative list stays
     a present [] key; zero rows => json!([])); CLI (--help lists search; search --help ok;
     missing pattern => code="usage"; dead port => code="connection"). ONE commit touching
     ONLY that file; house-red via use oh_my_ib::ib::{shape_search, SearchRow}. Hash = spec-rev.
  2. RECORD COMMIT: tasks/01.md (status todo, attempts 0,
     verify=["cargo build","cargo test --test search_command"],
     spec-paths=[tests/search_command.rs],
     impl-paths=[src/cli.rs, src/ib/search.rs, src/ib/mod.rs, src/main.rs],
     spec-rev=<freeze hash>); current.json stage=task + full-verify=["cargo build","cargo test"];
     journal seq=3. Metadata only.
Feature gotchas:
  - NEVER run repo-wide cargo fmt — fmt src/** only. Public repo: no secrets.
  - No STK guard in search (D3) — the frozen CLI tests must NOT expect a sec-type error.
  - main.rs IS in impl-paths this time (new dispatch arm) — unlike the last two features.
  - spec/impl paths disjoint; single card, single freeze commit.
Done when: both commits pushed, card 01 todo, journal seq=3 appended.
On success: run pipeline-impl (operator hands to interactive π/omp).
On failure: attempts++; >=3 => blocked => run pipeline-hunt.
<<< END
