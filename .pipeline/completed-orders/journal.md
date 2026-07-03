# journal — completed-orders (append-only)

## seq=1 · 2026-07-03T11:07:05Z · prd→arch · completed · by=claude-fable-5
done:   PRD decision-complete: omi completed-orders via reqCompletedOrders(api_only=false);
        operator locked D1 (feature over FX-quote/account-summary-tags) and D2 (team rotation:
        keep pi=impl, codex=review this round; paradigm = roles rotate, every stage SOTA,
        KB note 41.100). Code facts: CompletedOrdersEnd shared-channel marker = orders.rs
        drain-to-End verbatim; NOT ADR 0012 class.
output: .pipeline/completed-orders/PRD.md
--- handoff ---
>>> NEXT
Run pipeline-arch on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
Model: frontier SOTA required — operator assigns the bot.
First: git pull --rebase; no .env in this repo.
Read for context (before acting):
  - AGENTS.md + CLAUDE.md; .pipeline/completed-orders/PRD.md (D1-D6)
  - src/ib/orders.rs — the sibling to mirror VERBATIM (drain, filter-when-set, row style)
  - ~/.cargo/registry/src/*/ibapi-3.1.0/src/orders/ — OrderData/OrderState/Orders enum +
    completed_orders (sync.rs:108-115) + shared_channel_configuration.rs:45 (End marker)
Your task (concrete, numbered):
  1. Pin the completed-row key set from ibapi source: open-orders 10-key parity + which
     completion fields exist on OrderData/OrderState (status/completed_time/completed_status/
     filled quantity/avg price?) and their absent-value mapping (house null rules).
  2. Write arch.md: src/ib/completed_orders.rs component (pure row seam for the frozen spec +
     gateway drain fn), cli/mod/main wiring, --account filter parity with orders.rs.
  3. Write ADR 0015 (repo-global numbering: 0014 taken) — completed-orders read class
     (drain-to-End, api_only=false, OrderData-only arm), row-shape decision.
  4. CONTEXT.md — glossary deltas.
  5. Pin freeze coverage: frozen = pure row seam + CLI contract; review-by-reading = the
     drain fn (mirror check vs orders.rs); live = PRD criterion 8 (shape/exit on any day).
Feature gotchas (project-specific traps the next node MUST know):
  - READ-ONLY red line: no place/modify/cancel code anywhere (public repo, trading gated).
  - NEVER repo-wide cargo fmt; never touch tests/. Public repo: no account ids/secrets.
  - NOT ADR 0012's markerless class — no TAKE_FIRST_TIMEOUT.
  - brief is FROZEN at 8 top-level keys — do NOT add a section.
  - ADR numbering is repo-global — next is 0015.
Done when: arch.md + CONTEXT.md + docs/adr/0015-*.md committed (stage=arch, journal seq=2,
one commit, pushed). On success: run pipeline-task.
On failure: attempts++; >=3 => blocked => run pipeline-hunt.
<<< END

## seq=2 · 2026-07-03T11:09:11Z · arch→task · completed · by=claude-fable-5
done:   arch complete: 4-file diff (cli CompletedOrders variant, NEW src/ib/completed_orders.rs
        with CompletedOrderRow + shape_completed_orders + drain fn, mod/main wiring); ADR 0015
        (drain-to-End, api_only=false hardcoded, 14-key row = open-orders 10-key parity + 4
        completion keys, avg_fill_price deliberately excluded, filter-when-set inline).
        Freeze coverage pinned. ibapi claims source-verified.
output: .pipeline/completed-orders/arch.md, .pipeline/completed-orders/CONTEXT.md,
        .pipeline/completed-orders/docs/adr/0015-completed-orders-drain.md
--- handoff ---
>>> NEXT
Run pipeline-task on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
Model: frontier SOTA required — operator assigns the bot.
First: git pull --rebase; no .env in this repo.
Read for context (before acting):
  - AGENTS.md + CLAUDE.md; .pipeline/completed-orders/{PRD.md,arch.md,docs/adr/0015-*.md}
  - tests/search_command.rs — freshest house freeze pattern (plain-row fixtures + house-red)
Your task (concrete, numbered):
  1. FREEZE COMMIT: write tests/completed_orders_command.rs per arch.md §Freeze coverage —
     shape_completed_orders (exact 14-key row; gateway order; None limit/aux -> null; ""
     completed_time pass-through; zero rows => json!([])); CLI (--help lists completed-orders;
     completed-orders --help ok; dead port => code="connection"). ONE commit touching ONLY
     that file; house-red via use oh_my_ib::ib::{shape_completed_orders, CompletedOrderRow}.
     Hash = spec-rev.
  2. RECORD COMMIT: tasks/01.md (todo, attempts 0,
     verify=["cargo build","cargo test --test completed_orders_command"],
     spec-paths=[tests/completed_orders_command.rs],
     impl-paths=[src/cli.rs, src/ib/completed_orders.rs, src/ib/mod.rs, src/main.rs],
     spec-rev=<freeze hash>); current.json stage=task + full-verify; journal seq=3.
Feature gotchas:
  - READ-ONLY red line; NEVER repo-wide cargo fmt; public repo no secrets.
  - The 14 keys and their order-of-definition are in arch.md §Component design — freeze the
    KEY SET (sorted assert), not serialization order (serde_json = BTreeMap).
  - No TAKE_FIRST_TIMEOUT anywhere near this drain.
Done when: both commits pushed, card 01 todo, journal seq=3 appended.
On success: run pipeline-impl (operator hands to interactive pi/omp — rotation kept per PRD D2).
On failure: attempts++; >=3 => blocked => run pipeline-hunt.
<<< END

## seq=3 · 2026-07-03T11:10:39Z · task→impl · completed · by=claude-fable-5
done:   spec frozen: tests/completed_orders_command.rs (9 tests, house-red via unresolved
        shape_completed_orders/CompletedOrderRow imports) @ spec-rev
        aff35991c759a0bbfd44dd03ea1d67fac0241dbf (freeze commit, spec-paths only); card 01
        recorded (todo, card-scoped verify, spec/impl disjoint, main.rs in impl-paths).
output: tests/completed_orders_command.rs, .pipeline/completed-orders/tasks/01.md
--- handoff ---
>>> NEXT
Run pipeline-impl on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
Model: capable-local OK (impl only) — operator assigns the bot (this run: interactive pi/omp,
rotation kept per PRD D2).
First: git pull --rebase; no .env in this repo.
Read for context (before acting):
  - AGENTS.md + CLAUDE.md — repo conventions
  - .pipeline/completed-orders/tasks/01.md — YOUR card (scope, hard constraints, freeze coverage)
  - .pipeline/completed-orders/arch.md — §Component design is the verbatim implementation
  - .pipeline/completed-orders/docs/adr/0015-*.md + PRD.md + CONTEXT.md
  - src/ib/orders.rs — the sibling whose drain/filter semantics you mirror
  - tests/completed_orders_command.rs — the FROZEN spec to turn green (read-only for you!)
Your task (concrete, numbered):
  1. git checkout -b feat/completed-orders (cut from current trunk main).
  2. Implement EXACTLY the card's impl-paths: src/cli.rs (CompletedOrders variant),
     src/ib/completed_orders.rs NEW (CompletedOrderRow + shape_completed_orders + drain fn),
     src/ib/mod.rs (mod + re-export), src/main.rs (dispatch arm).
  3. Verify: cargo build && cargo test --test completed_orders_command (red->green), then
     cargo test (full suite green) and cargo clippy --all-targets -- -D warnings.
  4. Freeze-gate self-check BEFORE committing:
     git diff aff35991c759a0bbfd44dd03ea1d67fac0241dbf HEAD -- tests/completed_orders_command.rs
     must print NOTHING; git status must show no tests/ changes.
  5. Commit, push branch, open PR: gh pr create --base main --head feat/completed-orders
     --title "feat(completed-orders): omi completed-orders — terminal order states (ADR 0015)"
     --body pointing at .pipeline/completed-orders/{PRD.md,arch.md,docs/adr/0015-*.md} + card 01.
  6. Metadata commit on MAIN (not the branch): card 01 todo->review, current.json stage=impl
     + pr URL, append journal seq=4, push main.
Feature gotchas (project-specific traps the next node MUST know):
  - READ-ONLY red line: no place/modify/cancel code anywhere (public repo, trading gated).
  - NEVER touch tests/ (frozen). NEVER repo-wide cargo fmt — fmt src/** only or skip.
  - api_only=false HARDCODED (no flag). Filter rows ONLY when cfg.account set (orders parity).
  - Orders::OrderData arm only; drain self-terminates on CompletedOrdersEnd — no explicit
    break, no TAKE_FIRST_TIMEOUT.
  - Debug-render action/tif/status (format!("{:?}")); limit/aux prices raw Option (None->null,
    NOT pnl_number).
  - brief.rs/orders.rs untouched. Metadata commit on MAIN; code diff on feat/completed-orders.
  - Public repo: no account ids/tokens/balances anywhere.
Done when: PR open, card verify + full suite + clippy green, freeze-gate diff empty,
card=review, journal seq=4 pushed. On success: run pipeline-review (operator hands to codex cli).
On failure: attempts++; >=3 => blocked => run pipeline-hunt.
<<< END

## seq=4 · 2026-07-03T11:18:26Z · task→impl · completed · by=interactive-π/glm-5.2
done:   impl landed on feat/completed-orders (PR #14): cli.rs CompletedOrders variant, NEW
        completed_orders.rs (CompletedOrderRow + shape_completed_orders pure seam + drain fn
        mirroring orders.rs), mod.rs mod+re-export, main.rs dispatch arm. 4-file diff, +109.
        Frozen spec untouched (freeze-gate diff empty). api_only=false hardcoded; --account
        filter-when-set; OrderData-only arm; drain self-terminates on CompletedOrdersEnd (NOT
        TAKE_FIRST_TIMEOUT); Debug-render action/tif/status; limit/aux raw Option (None->null).
output: PR #14 (https://github.com/jackypanster/oh-my-ib/pull/14), feat/completed-orders tip a4d1fe6
--- handoff ---
>>> NEXT
Run pipeline-review on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=feat/completed-orders pr=https://github.com/jackypanster/oh-my-ib/pull/14
Model: frontier SOTA required — operator assigns the bot (this run: codex cli, rotation kept per PRD D2).
First: git pull --rebase; no .env in this repo.
Read for context (before acting):
  - AGENTS.md + CLAUDE.md — repo conventions (public repo, READ-ONLY, agent-first docs)
  - .pipeline/completed-orders/tasks/01.md — the card (status=review): scope, hard constraints, freeze coverage
  - .pipeline/completed-orders/arch.md — §Component design is what impl followed verbatim
  - .pipeline/completed-orders/docs/adr/0015-completed-orders-drain.md — binding decisions
  - .pipeline/completed-orders/PRD.md + CONTEXT.md — criteria + glossary
  - src/ib/orders.rs — the sibling whose drain/filter SEMANTICS impl mirrors (drain shape differs: no _with_client seam; filter inline — same semantics, simpler shape, ADR 0015 records why)
  - PR #14 diff (gh pr diff 14) — the review surface
Your task (concrete, numbered):
  1. Freeze gate FIRST (deterministic): git diff aff35991c759a0bbfd44dd03ea1d67fac0241dbf <review-tip> -- tests/completed_orders_command.rs
     must be EMPTY. Non-empty ⇒ reject (attempts++, route impl; >=3 ⇒ hunt).
  2. Full-suite gate: checkout feat/completed-orders, run current.json.full-verify
     (["cargo build","cargo test"]) — must be GREEN. Red attributable to card 01 ⇒ flip
     review→todo, attempts++; cross-card with no single owner ⇒ reviews/integration-NN.md
     + route pipeline-hunt.
  3. Semantic review (by reading) of the impl diff vs arch.md §Component design + card freeze
     coverage: completed_orders(false) hardcoded call, OrderData-only arm, drain self-terminates
     on CompletedOrdersEnd (no explicit break, no TAKE_FIRST_TIMEOUT), filter-when-set parity
     with orders.rs (cfg.account set ⇒ filter; unset ⇒ pass-through), {"completed_orders": …}
     wrapper, contexts "completed-orders". Field mapping exact 14 keys: order_id/account/symbol
     (newtype .to_string())/conid/action (Debug)/quantity/order_type/limit_price (raw Option,
     None→null)/aux_price (raw Option)/tif (Debug)/status (Debug OrderStatusKind)/
     filled_quantity/completed_time (String pass-through)/completed_status (String). output.rs/
     error.rs/brief.rs/orders.rs untouched. No secrets.
  4. READ-ONLY red line grep: grep the diff for place/modify/cancel order API calls
     (.place_order/.what_if_order/.modify_order/.cancel_order/.reqGlobalCancel/ etc.) — must be
     ABSENT. (Doc comments mentioning \"cancelled\" as an order state are NOT code paths.)
  5. If all pass: operator live acceptance (PRD criterion 8): omi --live completed-orders exits 0
     with the wrapper shape; [] on a no-trade day is a PASS (row content rides the first active
     trading day). Then human-confirm + squash-merge PR #14 (the only merge).
  6. After human-confirm + merge: card 01 status review->done, current.json stage=done (drop pr?),
     append journal seq=5, push main.
Feature gotchas (project-specific traps the next node MUST know):
  - Freeze gate is the deterministic two-commit diff over tests/completed_orders_command.rs — empty = pass.
  - READ-ONLY red line: the diff contains request/drain/emit only — no place/modify/cancel code paths.
    Doc comments mentioning \"cancelled\"/\"cancels\" are adjective/verb in prose, NOT trading API calls.
  - Drain self-terminates on CompletedOrdersEnd by design — do NOT flag the absence of TAKE_FIRST_TIMEOUT
    or an explicit break as a bug (ADR 0015; CompletedOrdersEnd is the End marker, ADR 0012 boundary).
  - api_only=false is HARDCODED with NO flag by design (operator trades via Tiger app; API-only view is
    empty — ADR 0015 D4). Do NOT flag the missing flag as an absent option.
  - Filter-when-set semantics: cfg.account set ⇒ filter; unset ⇒ pass-through (never auto-filter to
    the resolved account, ADR 0011). The filter is inline (no _with_client seam) because completed-
    orders has no brief consumer — review checks SEMANTICS parity with orders.rs, not code-shape identity.
  - average_fill_price deliberately excluded (lives on Orders::OrderStatus, not OrderData; executions
    carries fill prices). 14-key row = open-orders 10-key parity + 4 completion keys.
  - The metadata commit is already on MAIN (this entry rides it); the code diff is on feat/completed-orders.
  - Public repo: scan the PR diff for account ids/tokens/balances before merge.
Done when: PR #14 squash-merged after freeze gate empty + full-suite green + semantic review pass
+ READ-ONLY grep clean + operator live acceptance (criterion 8); card=done, journal seq=5 pushed.
On failure: attempts++; >=3 ⇒ blocked ⇒ run pipeline-hunt.
<<< END

## seq=5 · 2026-07-03T11:28:38Z · impl→impl · failed · by=claude-fable-5
done:   LIVE ACCEPTANCE FAILED (pre-review): omi --live completed-orders hung 2x on a healthy
        gateway (>2.5min, >45s; health OK between; first run of the session — no pollution).
        CompletedOrdersEnd never arrived = known upstream class (ib_insync #224; twsapi
        groups.io reports). NOT an impl bug (round 1 implemented ADR 0015 verbatim) — arch
        drain posture gap. Routed: ADR 0016 (bounded per-item drain, timing-classified None,
        reuses TAKE_FIRST_TIMEOUT + timeout/exit-6 — all operator-locked decisions), arch.md
        Amendment, PRD criterion-8 amendment, card 01 review->todo attempts=1 with Round 2
        scope. Frozen spec UNCHANGED (no re-freeze; drain is review-by-reading).
output: docs/adr/0016-bounded-drain-completed-orders.md, arch.md §Amendment,
        PRD.md §Amendment, tasks/01.md Round 2
--- handoff ---
>>> NEXT
Run pipeline-impl ROUND 2 on the SAME branch (feat/completed-orders, PR #14 open).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=https://github.com/jackypanster/oh-my-ib/pull/14
Model: capable-local OK (impl only) — operator assigns the bot (interactive pi/omp).
First: git pull --rebase on main (get ADR 0016 + amendments), then git checkout feat/completed-orders.
Read for context (before acting):
  - .pipeline/completed-orders/tasks/01.md §Round 2 — YOUR scope (drain loop ONLY)
  - .pipeline/completed-orders/arch.md §Amendment — the verbatim replacement loop
  - .pipeline/completed-orders/docs/adr/0016-*.md — why + the timing-classification rule
Your task (concrete, numbered):
  1. git pull --rebase (main); git checkout feat/completed-orders.
  2. Replace ONLY the drain loop in src/ib/completed_orders.rs per arch.md §Amendment
     (timeout_iter_data(TAKE_FIRST_TIMEOUT) + Instant-classified None arms). Nothing else.
  3. Verify: cargo build && cargo test --test completed_orders_command && cargo test &&
     cargo clippy --all-targets -- -D warnings.
  4. Freeze gate self-check: git diff aff35991c759a0bbfd44dd03ea1d67fac0241dbf HEAD --
     tests/completed_orders_command.rs must be EMPTY.
  5. Commit on the branch ("fix(completed-orders): bound the drain per ADR 0016"), push
     (updates PR #14).
  6. Metadata commit on MAIN: card 01 todo->review (attempts stays 1), current.json
     stage=impl, journal seq=6, push main.
Feature gotchas: READ-ONLY red line unchanged; never touch tests/; no repo-wide fmt;
TAKE_FIRST_TIMEOUT is pub in src/ib/mod.rs (reuse, do NOT redefine); public repo no secrets.
Done when: PR #14 updated, all verify green, freeze gate empty, card=review, seq=6 pushed.
On success: run pipeline-review (codex cli). On failure: attempts++ (=2); >=3 => blocked => hunt.
<<< END
