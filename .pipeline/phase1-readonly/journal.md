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

## seq=2 · 2026-06-28T07:40:04Z · arch→task · completed · by=claude-opus-4-8(claude-code)
done:   Locked the architecture: lib(oh_my_ib)+bin(omi) split so the freeze gate can protect the
        black-box CLI contract (incl. an offline-deterministic connection-error envelope), while the
        live ib/** calls are reviewed-by-reading + manual paper acceptance. Wrote arch.md, CONTEXT.md,
        and ADRs 0001-0006. ibapi sync surface confirmed (Client at ibapi::client::blocking::Client).
output: .pipeline/phase1-readonly/arch.md, CONTEXT.md, docs/adr/0001..0006, .pipeline/current.json
--- handoff ---
>>> NEXT
Run pipeline-task on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
First: git pull --rebase; load repo config (.env if present, per CONTRACT step 2).
Read for context (before acting):
  - oh-my-ib/CLAUDE.md           — project conventions + hard safety rules (read FIRST)
  - .pipeline/phase1-readonly/arch.md      — crate shape, component->card map, freeze coverage
  - .pipeline/phase1-readonly/CONTEXT.md   — domain glossary (ports 4002/4001, md-type, conId, ...)
  - .pipeline/phase1-readonly/docs/adr/*   — ADR 0006 defines what IS frozen
Your task (concrete, numbered):
  1. Decompose into TWO cards (arch.md "component boundaries" maps to them):
     - card 01 = core: crate scaffold + clap CLI skeleton + global flags + `health` subcommand +
       config + error/output envelope. Frozen spec tests/cli_contract.rs.
     - card 02 = the six read subcommands (account/positions/orders/quote/contract/history) wired to
       ibapi. Frozen spec tests/data_commands.rs.
  2. Freeze ALL feature tests in ONE commit (CONTRACT spec-rev double-commit): write tests/cli_contract.rs
     + tests/data_commands.rs as BLACK-BOX subprocess tests via assert_cmd::Command::cargo_bin("omi").
     They reference ONLY binary name + args + stdout/stderr/exit — no internal symbols.
  3. Freeze coverage (greenfield): at freeze time there is NO crate yet, so `cargo test` is RED
     (does not compile / no Cargo.toml). That IS the red state. impl creates Cargo.toml + src to go green.
     Record this in each card's `## Freeze coverage`.
  4. Card-scoped verify (CONTRACT): card 01 -> `cargo test --test cli_contract`; card 02 ->
     `cargo test --test data_commands` (+ `cargo build` in each). Set current.json.full-verify =
     ["cargo build", "cargo test"] (whole-suite runner for review's final gate).
  5. impl-paths: card 01 = Cargo.toml, src/{main,lib,cli,config,output,error,model}.rs, src/ib/{mod,client,account... but account is card 02}. Keep spec-paths (tests/*) disjoint from impl-paths.
Frozen-test assertions to author (black-box, must FAIL on an empty repo, go green when impl builds it):
  cli_contract.rs:
   - `omi --help` stdout contains: health account positions orders quote contract history
   - `omi --version` exits success
   - `omi health --help` exits success
   - unknown subcommand `omi frobnicate` exits FAILURE
   - `omi --format json health --host 127.0.0.1 --port 65000` exits NON-ZERO and stderr parses as JSON
     with a top-level "error" object whose code == "connection"  (dead port => deterministic, no gateway)
  data_commands.rs:
   - `omi account --help`, `omi positions --help`, `omi orders --help`, `omi contract --help` exit success
   - `omi quote --help` stdout mentions `--md-type`
   - `omi history --help` stdout mentions `--bar` and `--duration`
   - `omi quote AAPL --port 65000 --format json` exits NON-ZERO with the JSON error envelope (dead port)
Feature gotchas:
  - GREENFIELD: empty repo, no Cargo project yet. assert_cmd needs Cargo.toml [[bin]] name="omi" +
    [dev-dependencies] assert_cmd, predicates — those are impl-paths (impl creates them).
  - Repo is PUBLIC: no account ids/secrets in any committed file or test.
  - Phase 1 read-only: do NOT author any order-placement test or card.
  - Use a dead port (65000) for error-path tests so they never depend on a running gateway.
Done when: freeze commit (tests only) + record commit (cards 01,02 + current.json{stage:task,full-verify}
  + journal) on main. On success: run pipeline-impl (it picks card 01 first).
On failure: attempts++; >=3 ⇒ blocked ⇒ run pipeline-hunt.
<<< END

## seq=3 · 2026-06-28T07:43:35Z · task→impl · completed · by=claude-opus-4-8(claude-code)
done:   Froze the spec in two ordered commits. Freeze commit 13e522d (tests/cli_contract.rs +
        tests/data_commands.rs only) = the feature's single spec-rev; confirmed RED (no crate yet).
        Record commit writes cards 01+02 (status=todo, spec-rev=13e522d), current.json{stage:task,
        full-verify=[cargo build, cargo test]}.
output: .pipeline/phase1-readonly/tasks/01.md, tasks/02.md, .pipeline/current.json
--- handoff ---
>>> NEXT
Run pipeline-impl on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
First: git pull --rebase; load repo config (.env if present, per CONTRACT step 2).
Read for context (before acting):
  - oh-my-ib/CLAUDE.md          — conventions + hard safety rules (read FIRST)
  - .pipeline/phase1-readonly/arch.md, CONTEXT.md, docs/adr/*  — shape + glossary + locked decisions
  - .pipeline/phase1-readonly/tasks/01.md  — the OLDEST todo card; pick this first
  - .pipeline/phase1-readonly/tasks/02.md  — next card (same branch/PR)
roles.yaml: impl slot = goal-driven-impl-claude (Claude-native; NOT the Hermes goal skill).
Your task (concrete, numbered):
  1. Create branch feat/phase1-readonly from main; flip card 01 status->in-progress, commit that flip to main.
  2. Implement card 01 inside its impl-paths (Cargo.toml + src/**) until `cargo build` and
     `cargo test --test cli_contract` both exit 0. NEVER edit tests/ (spec-paths, frozen).
  3. Verify the ibapi 3.1 SYNC api on docs.rs before writing src/ib/** (type
     ibapi::client::blocking::Client; connect("host:port", client_id); account_summary / positions /
     open_orders / market_data builder snapshot / contract_details / historical_data). Do NOT guess names.
  4. Green ⇒ push feat/phase1-readonly, open PR via gh, then on main flip card 01 status->review,
     set current.json.stage=impl, append journal — ONE commit on main. Then run pipeline-impl for card 02.
  5. Card 02: implement the six subcommands until `cargo test --test data_commands` is green (same branch).
     When BOTH cards are status=review, hand off to pipeline-review (NOT before).
Feature gotchas:
  - GREENFIELD: you create Cargo.toml + all of src/. dev-deps: assert_cmd, predicates, serde_json.
  - Make global flags (--format/--host/--port/--client-id/--account/--md-type/--live/--timeout)
    clap `global = true` so they parse before AND after the subcommand (the frozen tests rely on this:
    e.g. `omi --format json health --host .. --port 65000`).
  - quote --help MUST contain `--md-type`; history --help MUST contain `--bar` and `--duration`.
  - Connection failure (dead port) MUST print `{"error":{"code":"connection",..}}` to stderr + exit!=0.
  - Repo is PUBLIC + Phase 1 read-only: no secrets committed, no order-placement code.
  - cargo test --test cli_contract is CARD-SCOPED; data_commands stays red until card 02 — that is expected.
Done when: both cards status=review, feat/phase1-readonly pushed with an open PR, current.json.stage=impl.
On failure of a card: attempts++; <3 ⇒ card todo + retry pipeline-impl; >=3 ⇒ blocked ⇒ pipeline-hunt.
<<< END

## seq=4 · 2026-06-28T07:59:28Z · impl→review · completed · by=claude-opus-4-8(claude-code)
done:   Implemented BOTH cards in one continuous impl run on feat/phase1-readonly (single operator,
        so the transient in-progress flips were collapsed). Built the omi crate (lib+bin), 7 read-only
        subcommands, JSON error envelope, ibapi sync ib/** layer. Frozen specs go green and are
        byte-intact (git diff 13e522d..branch -- tests/ is empty). cargo build + clippy -D warnings +
        cargo test (7 unit + 5 cli_contract + 7 data_commands) all pass OFFLINE. Cards 01+02 → review.
        PR #1 opened. STOPPED before merge per operator (merge needs human confirm + paper acceptance).
output: feat/phase1-readonly @ 9bd8e4e, PR https://github.com/jackypanster/oh-my-ib/pull/1,
        tasks/01.md tasks/02.md (status=review), current.json (stage=impl, pr set)
--- handoff ---
>>> NEXT
Run pipeline-review on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=https://github.com/jackypanster/oh-my-ib/pull/1
First: git pull --rebase; load repo config (.env if present, per CONTRACT step 2).
Read for context (before acting):
  - oh-my-ib/CLAUDE.md, .pipeline/phase1-readonly/{PRD,arch,CONTEXT}.md, docs/adr/0006 (freeze coverage)
  - tasks/01.md, tasks/02.md (both status=review), the PR diff (gh pr diff 1)
roles.yaml: review slot = check. Forge = github → use gh.
Your task (concrete, numbered):
  1. Freeze gate (deterministic): `git fetch origin && git diff 13e522dc70a432b0403cd75d4b5b82531a77a6fa origin/feat/phase1-readonly -- tests/` MUST be empty. Non-empty ⇒ reject (attempts++, route impl/task).
  2. Full-verify on the branch tip (current.json.full-verify): checkout origin/feat/phase1-readonly,
     run `cargo build` then `cargo test` — must be GREEN (it is offline; no gateway needed for the suite).
  3. Semantic review by READING (ADR 0006 — ib/** is not freeze-covered): src/ib/** correctness,
     error mapping, no order-placement code (read-only), no secrets, paper-default safety.
     Known first-cut: src/ib/orders.rs serializes orders via debug form — flag for a follow-up card if desired.
  4. HARD GATE — do NOT squash-merge until the OPERATOR confirms manual paper-account acceptance
     (PRD criteria 1-8 against a running paper IB Gateway on :4002). The offline suite does NOT prove
     live data correctness. Only pipeline-review merges, only after explicit human confirm.
On merge (after human confirm): squash-merge PR #1, set every card status=done, current.json.stage=done,
  append a journal entry. Then Phase 1 is shipped.
On reject: attempts++; route the offending card to pipeline-impl (or pipeline-task to re-freeze if the
  spec itself is wrong); >=3 ⇒ blocked ⇒ pipeline-hunt.
<<< END

## seq=5 · 2026-06-28T09:14:58Z · review→impl · failed · by=hermes-gpt-5.5

done:   Reviewed PR #1 at head 9bd8e4e. Freeze gate passed and full offline verify passed
        (`cargo build`, `cargo test`, `cargo clippy --all-targets -- -D warnings`), but semantic review
        found contract blockers in global `--md-type` handling and account/positions JSON shaping.
output: .pipeline/phase1-readonly/reviews/review-01.md; tasks/01.md and tasks/02.md set back to todo
--- handoff ---
>>> NEXT
Run pipeline-impl on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=https://github.com/jackypanster/oh-my-ib/pull/1
First: git pull --rebase; load repo config (.env if present, per CONTRACT step 2).
Read for context (before acting):
  - oh-my-ib/CLAUDE.md — repo conventions + hard read-only/public-repo safety rules
  - .pipeline/phase1-readonly/{PRD.md,arch.md,CONTEXT.md} and docs/adr/0006-lib-bin-split-freeze-coverage.md
  - .pipeline/phase1-readonly/reviews/review-01.md — exact review blockers + verification evidence
  - .pipeline/phase1-readonly/tasks/01.md and tasks/02.md — both status=todo, attempts=1
Your task (concrete, numbered):
  1. Reuse/update PR branch feat/phase1-readonly; do not edit frozen spec paths tests/cli_contract.rs or tests/data_commands.rs.
  2. Card 01 fix: make `--md-type live|delayed|frozen` a GLOBAL flag with flag>toml>default precedence. `omi --md-type delayed quote AAPL --port 65000 --format json` must parse and then hit the structured connection envelope on the dead port.
  3. Card 02 fix: shape `account` JSON to stable documented keys (`account`, `net_liquidation`, `total_cash`, `buying_power`, `available_funds`, `currency`) and shape `positions` JSON to documented keys (`symbol`, `conid`, `qty`, `avg_cost`, `market_value`, `unrealized_pnl`). If ibapi `positions()` cannot provide market_value/unrealized_pnl, route to pipeline-task for a re-spec instead of guessing.
  4. Keep Phase 1 structurally read-only: no place/modify/cancel order path; no secrets/account ids in repo.
  5. Verify card-scoped commands (`cargo build`, `cargo test --test cli_contract`, `cargo test --test data_commands`) and then full `cargo build && cargo test`; clippy with `--all-targets -- -D warnings` should remain green.
Feature gotchas:
  - Freeze gate already passed for PR head 9bd8e4e; after fixes, review will re-run `git diff 13e522d..HEAD -- tests/` and it must still be empty.
  - `cargo fmt --check` currently reports formatting drift only in frozen trunk spec tests; do not fix it from impl unless the operator explicitly routes a re-freeze/task change.
  - Orders debug-string JSON was noted as a follow-up/advisory, not the rejection reason.
Done when: both cards are back to status=review, PR #1 updated, and handoff returns to pipeline-review.
On failure: attempts++; attempts >=3 ⇒ blocked ⇒ run pipeline-hunt.
<<< END

## seq=6 · 2026-06-28T09:19:06Z · review→impl · failed · by=hermes-gpt-5.5

done:   Added delegated-review addendum after the parallel Rust/CLI reviewers completed. This does not
        increment attempts again; it enriches the same PR #1 rejection with additional blockers:
        live-port opt-in bypass, ignored account scoping, clap parse errors outside JSON envelope,
        no-op timeout, `default_account` config mismatch, and debug-string orders/history output.
output: .pipeline/phase1-readonly/reviews/review-02.md; tasks/01.md and tasks/02.md rejection notes expanded
--- handoff ---
>>> NEXT
Run pipeline-impl on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=https://github.com/jackypanster/oh-my-ib/pull/1
First: git pull --rebase; load repo config (.env if present, per CONTRACT step 2).
Read for context (before acting):
  - oh-my-ib/CLAUDE.md — repo conventions + hard read-only/public-repo/live-port safety rules
  - .pipeline/phase1-readonly/{PRD.md,arch.md,CONTEXT.md} and docs/adr/0006-lib-bin-split-freeze-coverage.md
  - .pipeline/phase1-readonly/reviews/review-01.md and review-02.md — complete rejection evidence
  - .pipeline/phase1-readonly/tasks/01.md and tasks/02.md — both status=todo, attempts=1
Your task (concrete, numbered):
  1. Reuse/update PR branch feat/phase1-readonly; do not edit frozen spec paths tests/cli_contract.rs or tests/data_commands.rs.
  2. Card 01 fix list: global `--md-type` with precedence; structured JSON envelope for clap parse failures; no silent live port via `--port 4001` or config without explicit `--live`; implement/remove `--timeout`; support documented `default_account` config key.
  3. Card 02 fix list: honor `--account`/configured account for account-scoped commands; stable account JSON keys; stable positions JSON keys; replace orders/history Debug strings with structured JSON. If `positions()` cannot provide market_value/unrealized_pnl, route to pipeline-task for re-spec.
  4. Keep Phase 1 structurally read-only: no place/modify/cancel order path; no secrets/account ids in repo.
  5. Verify card-scoped commands (`cargo build`, `cargo test --test cli_contract`, `cargo test --test data_commands`) and then full `cargo build && cargo test`; clippy with `--all-targets -- -D warnings` should remain green.
Feature gotchas:
  - Freeze gate must remain empty: `git diff 13e522dc70a432b0403cd75d4b5b82531a77a6fa HEAD -- tests/`.
  - `cargo fmt --check` reported formatting drift only in frozen trunk spec tests; do not fix frozen tests from impl unless routed through task/re-freeze.
  - This seq=6 is an addendum to the same rejection, not a second attempts increment.
Done when: both cards are back to status=review, PR #1 updated, and handoff returns to pipeline-review.
On failure: attempts++; attempts >=3 ⇒ blocked ⇒ run pipeline-hunt.
<<< END

## seq=7 · 2026-06-28T12:46:53Z · impl→review · completed · by=claude-opus-4-8(claude-code)
done:   Fixed every review-01 + review-02 blocker on feat/phase1-readonly (commit 5545b56).
        Card 01: global --md-type w/ flag>toml>default; clap parse errors → JSON envelope (code=usage,
        exit 64); live port 4001 refused without --live; default_account config key (alias account);
        removed no-op --timeout. Card 02: account/positions now use account_updates (account-scoped,
        real market_value/unrealized_pnl — resolved BLOCKER 3 in code, NO re-spec needed); stable
        snake_case keys; orders/history structured JSON; --account honored; quote/contract guard
        non-STK + honor --exchange/--currency. Freeze gate empty (git diff 13e522d -- tests/).
        Verified offline: cargo build + clippy -D warnings + cargo test (12 unit + 5 cli_contract +
        7 data_commands). Both cards → review (attempts stays 1; this is the fix for the same reject).
output: feat/phase1-readonly @ 5545b56, PR #1 updated; tasks/01.md+02.md (status=review + Fix notes),
        arch.md (usage error row), current.json (stage=impl)
--- handoff ---
>>> NEXT
Run pipeline-review on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=https://github.com/jackypanster/oh-my-ib/pull/1
First: git pull --rebase; load repo config (.env if present, per CONTRACT step 2).
Read for context (before acting):
  - .pipeline/phase1-readonly/reviews/review-01.md + review-02.md (the rejection list to re-verify)
  - .pipeline/phase1-readonly/tasks/01.md + 02.md "## Fix attempt 1" notes (what changed)
  - PR #1 diff at head 5545b56 (gh pr diff 1)
Your task (concrete, numbered):
  1. Freeze gate: `git fetch origin && git diff 13e522dc70a432b0403cd75d4b5b82531a77a6fa origin/feat/phase1-readonly -- tests/` MUST be empty.
  2. Full verify on origin/feat/phase1-readonly: `cargo build` && `cargo test` GREEN (offline); `cargo clippy --all-targets -- -D warnings` clean.
  3. Re-verify each prior blocker is closed by READING + (where offline-checkable) running:
     - global `omi --md-type delayed quote AAPL --port 65000 --format json` → JSON connection envelope (not clap error)
     - `omi frobnicate` → JSON {"error":{"code":"usage"}}, exit 64
     - `omi account --port 4001` (no --live) → JSON config error (live refused)
     - read src/ib/account.rs + positions.rs: stable keys + account_updates (market_value/unrealized_pnl present)
     - read src/ib/orders.rs + history.rs: structured JSON, no Debug strings; orders filters by --account
  4. HARD GATE — do NOT merge until the OPERATOR confirms manual paper-account acceptance (PRD 1–8 on a
     live paper gateway :4002). Offline suite cannot prove live values. Only pipeline-review merges, human-confirmed.
On merge (after human confirm): squash-merge PR #1, set both cards status=done, current.json.stage=done, append journal.
On reject: attempts++ (=2); route to pipeline-impl; attempts>=3 ⇒ blocked ⇒ pipeline-hunt.
<<< END

## seq=8 · 2026-06-28T12:58:03Z · review→impl · failed · by=codex-gpt-5
done:   Reviewed PR #1 at head 5545b56 after the review-01/02 fixes. Freeze gate passed; final
        full-suite gate on a detached PR worktree passed (`cargo build`, `cargo test`), and
        `cargo clippy --all-targets -- -D warnings` passed. Prior blockers were rechecked and closed,
        but semantic review found one remaining Card 01 CLI error-contract blocker: bare `omi` exits 0
        with help instead of a JSON usage error for the missing required subcommand.
output: .pipeline/phase1-readonly/reviews/review-03.md; tasks/01.md set to todo, attempts=2
--- handoff ---
>>> NEXT
Run pipeline-impl on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=https://github.com/jackypanster/oh-my-ib/pull/1
First: git pull --rebase; load repo config (.env if present, per CONTRACT step 2).
Read for context (before acting):
  - oh-my-ib/CLAUDE.md — read-only/public-repo/live-port safety rules
  - .pipeline/phase1-readonly/reviews/review-03.md — exact remaining blocker + verification evidence
  - .pipeline/phase1-readonly/tasks/01.md — status=todo, attempts=2; this is the retry target
  - .pipeline/phase1-readonly/tasks/02.md — still status=review; do not redo it unless your change touches it
Your task (concrete, numbered):
  1. Reuse PR branch feat/phase1-readonly; do not edit frozen spec paths tests/cli_contract.rs or tests/data_commands.rs.
  2. Fix Card 01 only: explicit `omi --help` and `omi --version` must remain successful, but bare `omi`
     (missing required subcommand) must not exit 0. Route it through the structured JSON usage envelope
     (`{"error":{"code":"usage",...}}`) with exit 64, or otherwise satisfy the non-zero error contract.
  3. If adding coverage, keep it in impl-owned source/unit tests; do not edit frozen `tests/cli_contract.rs`.
  4. Re-run `cargo build`, `cargo test --test cli_contract`, full `cargo test`, and
     `cargo clippy --all-targets -- -D warnings`.
Feature gotchas:
  - Review already re-verified the prior review-01/02 blockers at PR head 5545b56; keep those behaviors intact.
  - Card 02 remains review-ready; keep the diff scoped to `src/main.rs` / impl-owned coverage unless the fix truly needs more.
  - Freeze gate must stay empty for frozen spec paths; do not change `tests/cli_contract.rs` or `tests/data_commands.rs` from impl.
Done when: Card 01 is back to status=review, PR #1 is updated, and handoff returns to pipeline-review.
On failure: attempts++; attempts >=3 ⇒ blocked ⇒ run pipeline-hunt.
<<< END

## seq=9 · 2026-06-28T13:03:34Z · impl→review · completed · by=claude-opus-4-8(claude-code)
done:   Fixed review-03's lone blocker on feat/phase1-readonly (commit 4ab49f2). src/main.rs now only
        exits 0 for explicit --help/--version; a bare `omi` / missing subcommand emits the JSON usage
        envelope (code=usage) and exits 64. Behavior matrix verified (bare omi=64, --help/--version=0,
        frobnicate=64, health --help=0, quote-without-symbol=64). Freeze gate empty (git diff 13e522d
        -- tests/). cargo build + clippy -D warnings + cargo test (12 unit + 5 cli_contract +
        7 data_commands) green. Card 01 → review (attempts stays 2). Card 02 already review.
output: feat/phase1-readonly @ 4ab49f2, PR #1 updated; tasks/01.md (status=review + Fix attempt 2)
--- handoff ---
>>> NEXT
Run pipeline-review on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=https://github.com/jackypanster/oh-my-ib/pull/1
First: git pull --rebase; load repo config (.env if present, per CONTRACT step 2).
Read for context (before acting):
  - .pipeline/phase1-readonly/reviews/review-01..03.md (full rejection history to confirm each is closed)
  - .pipeline/phase1-readonly/tasks/01.md "## Fix attempt 2" + tasks/02.md "## Fix attempt 1"
  - PR #1 diff at head 4ab49f2 (gh pr diff 1)
Your task (concrete, numbered):
  1. Freeze gate: `git fetch origin && git diff 13e522dc70a432b0403cd75d4b5b82531a77a6fa origin/feat/phase1-readonly -- tests/` MUST be empty.
  2. Full verify on origin/feat/phase1-readonly: `cargo build` && `cargo test` GREEN; `cargo clippy --all-targets -- -D warnings` clean.
  3. Re-verify review-03 closed: build it, then `./target/debug/omi` → exit 64 + {"error":{"code":"usage"}};
     `omi --help` exit 0; `omi --version` exit 0; `omi frobnicate` exit 64. Confirm review-01/02 behaviors intact.
  4. HARD GATE — do NOT merge until the OPERATOR confirms manual paper-account acceptance (PRD 1–8 on a
     paper gateway :4002). Only pipeline-review merges, human-confirmed.
On merge (after human confirm): squash-merge PR #1, set both cards status=done, current.json.stage=done, append journal.
On reject: Card 01 attempts→3 ⇒ blocked ⇒ pipeline-hunt (circuit breaker); Card 02 reject ⇒ attempts++ ⇒ impl.
<<< END

## seq=10 · 2026-06-28T13:11:16Z · review→review · completed · by=codex-gpt-5
done:   Reviewed PR #1 at head 4ab49f2 after the review-03 fix. Freeze gate empty; final full-suite
        gate passed on a detached PR worktree (`cargo build`, `cargo test`), and
        `cargo clippy --all-targets -- -D warnings` passed. Rechecked the bare `omi` fix plus prior
        review-01/02 blockers; no blocking findings remain. Verdict: approve, awaiting explicit human
        confirm plus paper-account acceptance.
output: .pipeline/phase1-readonly/reviews/review-04.md
--- handoff ---
>>> NEXT
Await explicit human confirmation before merge. Do NOT merge automatically.

Required confirmation:
  1. Operator has run/accepted PRD criteria 1-8 against a paper IB Gateway on port 4002.
  2. Operator authorizes pipeline-review to squash-merge PR #1.

If confirmed:
  1. First run `git pull --rebase`; re-read PR #1 state and ensure head is still
     4ab49f2d415ee1658aa04fb7fedf1b6150f061e7. If the head changed, re-run freeze gate and full verify.
  2. Squash-merge PR #1 via GitHub, deleting `feat/phase1-readonly`.
  3. On `main`, set both cards to `done`, set `.pipeline/current.json.stage` to `done`, append the
     `review→done` journal entry, commit, and push trunk metadata.

If rejected:
  - Name the offending card and reason. Card 01 rejection increments attempts to 3 and routes to
    pipeline-hunt; Card 02 rejection increments attempts to 2 and routes to pipeline-impl.
<<< END
