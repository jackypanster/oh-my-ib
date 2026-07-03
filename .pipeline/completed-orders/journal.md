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
