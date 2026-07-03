# journal — stk-orders (append-only)

## seq=1 · 2026-07-03T12:09:07Z · prd→arch · completed · by=claude-fable-5
done:   PHASE 2 OPENED (operator-authorized): first write path. PRD decision-complete:
        buy/sell/cancel verbs, LMT/MKT TIF=DAY, paper-free + live double-gate
        (--live AND OMI_ALLOW_LIVE=1, config error pre-connect, offline-frozen), bounded
        first-ack (shared 10s; timeout = UNKNOWN + never blind-retry), v1 = place+cancel
        (modify/notional-cap/dry-run explicitly deselected). Red-line docs amendment
        (AGENTS.md/CLAUDE.md) rides the PR. Acceptance on PAPER gateway (operator must log
        gateway into paper for criterion 11).
output: .pipeline/stk-orders/PRD.md
--- handoff ---
>>> NEXT
Run pipeline-arch on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
Model: frontier SOTA required — HIGHEST-STAKES feature to date (first write path); operator assigns.
First: git pull --rebase; no .env in this repo.
Read for context (before acting):
  - AGENTS.md + CLAUDE.md — the red line you are AMENDING (scope it precisely)
  - .pipeline/stk-orders/PRD.md — criteria 1-11, decisions D1-D5, non-scope list
  - ~/.cargo/registry/src/*/ibapi-3.1.0/src/orders/ — pin from source: place_order vs
    submit_order vs order builder; next_valid_order_id / order-id allocation; what the order
    subscription yields (OrderStatus/OpenOrder events — which is the FIRST ack); cancel_order
    call + its ack channel; routing domain (order-id domain per ADR 0010 table)
  - src/ib/pnl.rs + completed_orders.rs — bounded-wait house patterns (ADR 0012/0016)
  - src/config.rs — where the OMI_ALLOW_LIVE gate check lives (pre-connect, config-level)
Your task (concrete, numbered):
  1. Pin the ibapi write-call shapes from source (the PRD's ack design may reshape here —
     criteria stay). Decide place fn + first-ack event choice + cancel ack.
  2. Write arch.md: module split (place.rs or orders_write.rs), pure seams (order-building,
     ack-shaping — frozen-testable), gate-check helper placement, exact CLI arg structs,
     wiring; the AGENTS.md/CLAUDE.md amendment TEXT (verbatim, so impl copies it).
  3. Write ADR 0017 (numbering: 0016 taken) — the write-path safety architecture (double
     gate, bounded ack, no-retry, paper-first acceptance); note the review-polarity flip
     (writes must exist ONLY in the new module, unreachable from reads).
  4. CONTEXT.md — glossary (write path, double gate, first ack, UNKNOWN state).
  5. Pin freeze coverage: frozen = seams + gate matrix (3 verbs x missing-gate, offline) +
     arg validation + dead-port + help; review-by-reading = gateway fns + docs amendment;
     live = criterion 11 PAPER lifecycle.
Feature gotchas (project-specific traps the next node MUST know):
  - This PR may touch AGENTS.md/CLAUDE.md — normally forbidden territory; ONLY the red-line
    paragraph, verbatim per arch.md, nothing else.
  - Gate check MUST precede connect (frozen tests rely on it being offline-deterministic).
  - Reuse TAKE_FIRST_TIMEOUT (pub, src/ib/mod.rs) for every ack wait — no new consts.
  - NEVER repo-wide cargo fmt; never touch tests/; public repo: no account ids/secrets.
  - ADR numbering repo-global — next is 0017.
Done when: arch.md + CONTEXT.md + docs/adr/0017-*.md committed (stage=arch, journal seq=2,
one commit, pushed). On success: run pipeline-task.
On failure: attempts++; >=3 => blocked => run pipeline-hunt.
<<< END

## seq=2 · 2026-07-03T12:14:37Z · arch→task · completed · by=claude-fable-5
done:   arch complete: write-path safety architecture (ADR 0017 — containment in trade.rs,
        effective-port double gate pre-connect, order-id-first bounded ack via ADR 0016
        pattern, no-retry UNKNOWN-state posture); pure seams build_stk_order (ibapi Order
        fields, frozen) + shape_order_ack (6-key); gate matrix fully offline-frozen; docs
        amendment text pinned verbatim; ibapi write calls source-verified
        (place_order/cancel_order subscriptions, PlaceOrder/CancelOrder enums,
        next_valid_order_id).
output: .pipeline/stk-orders/arch.md, CONTEXT.md, docs/adr/0017-write-path-safety.md
--- handoff ---
>>> NEXT
Run pipeline-task on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
Model: frontier SOTA required — highest-stakes freeze so far.
First: git pull --rebase; no .env in this repo.
Read for context (before acting):
  - AGENTS.md + CLAUDE.md; .pipeline/stk-orders/{PRD.md,arch.md,docs/adr/0017-*.md,CONTEXT.md}
  - tests/quote_ticks.rs — precedent: frozen tests MAY use ibapi types (dev-dependency)
  - tests/completed_orders_command.rs — freshest house freeze style
Your task (concrete, numbered):
  1. FREEZE COMMIT: write tests/stk_orders_command.rs per arch.md §Freeze coverage —
     build_stk_order LMT/MKT x buy/sell (assert ibapi Order fields: action, total_quantity,
     order_type, limit_price, tif Day + Contract symbol/STK); shape_order_ack 6-key exact +
     MKT null limit; GATE MATRIX offline (3 verbs x: --live no-env => config; --port 4001
     no-env => config; --live + OMI_ALLOW_LIVE=1 + dead port => connection; paper default +
     dead port => connection) using assert_cmd .env()/.env_remove(); validation (qty<=0,
     --limit<=0, missing args => usage); --help lists buy/sell/cancel. ONE commit touching
     ONLY that file; house-red via use oh_my_ib::ib::{build_stk_order, shape_order_ack}.
     Hash = spec-rev.
  2. RECORD COMMIT: tasks/01.md (todo, attempts 0,
     verify=["cargo build","cargo test --test stk_orders_command"],
     spec-paths=[tests/stk_orders_command.rs],
     impl-paths=[src/cli.rs, src/ib/trade.rs, src/ib/mod.rs, src/main.rs, AGENTS.md, CLAUDE.md],
     spec-rev=<freeze hash>); card body: scope + hard constraints + freeze coverage per
     arch.md (incl. the docs-amendment-verbatim rule and the containment grep); current.json
     stage=task + full-verify; journal seq=3.
Feature gotchas:
  - Gate check precedes connect — the env-based tests are fully offline; use .env_remove to
    guarantee a clean env per test.
  - Frozen tests construct ibapi Order/Contract types directly (quote_ticks precedent).
  - impl-paths includes AGENTS.md/CLAUDE.md (docs amendment) — unusual but arch-pinned.
  - NEVER repo-wide cargo fmt; public repo: no secrets.
Done when: both commits pushed, card 01 todo, journal seq=3 appended.
On success: run pipeline-impl (operator hands to interactive pi/omp; codex reviews after).
On failure: attempts++; >=3 => blocked => run pipeline-hunt.
<<< END

## seq=3 · 2026-07-03T12:16:53Z · task→impl · completed · by=claude-fable-5
done:   spec frozen: tests/stk_orders_command.rs (16 tests, house-red via unresolved
        build_stk_order/shape_order_ack imports; gate matrix fully offline via
        .env_remove/.env) @ spec-rev 3692c71bc11e873b2be8f3c9448a2a8d4f4d9e8f (freeze commit,
        spec-paths only); card 01 recorded (todo; impl-paths includes AGENTS.md/CLAUDE.md for
        the red-line amendment — arch-pinned verbatim).
output: tests/stk_orders_command.rs, .pipeline/stk-orders/tasks/01.md
--- handoff ---
>>> NEXT
Run pipeline-impl on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
Model: capable-local OK (impl only) — operator assigns (this run: interactive pi/omp).
First: git pull --rebase; no .env in this repo.
Read for context (before acting):
  - AGENTS.md + CLAUDE.md — you will AMEND their red-line bullet (arch.md §Docs amendment,
    VERBATIM, nothing else in those files)
  - .pipeline/stk-orders/tasks/01.md — YOUR card (scope/constraints/freeze coverage, exact)
  - .pipeline/stk-orders/arch.md — §Component design + §Docs amendment are the verbatim impl
  - .pipeline/stk-orders/docs/adr/0017-write-path-safety.md + PRD.md + CONTEXT.md
  - src/ib/completed_orders.rs — the ADR 0016 bounded-loop pattern you replicate
  - tests/stk_orders_command.rs — the FROZEN spec (read-only for you!)
Your task (concrete, numbered):
  1. git checkout -b feat/stk-orders (cut from current trunk main).
  2. Implement EXACTLY the card's impl-paths (cli.rs 3 variants, NEW src/ib/trade.rs with the
     two pure seams + gate + three gateway fns, mod.rs re-exports, main.rs arms,
     AGENTS.md/CLAUDE.md verbatim amendment).
  3. Ordering invariant the frozen tests assert: usage validation FIRST, then gate (config),
     then connect (connection). Gate = effective-port rule (cfg.port == LIVE_PORT).
  4. Verify: cargo build && cargo test --test stk_orders_command (16 red->green), then cargo
     test (full suite) and cargo clippy --all-targets -- -D warnings.
  5. Freeze-gate self-check: git diff 3692c71bc11e873b2be8f3c9448a2a8d4f4d9e8f HEAD --
     tests/stk_orders_command.rs must print NOTHING.
  6. Commit, push branch, open PR: gh pr create --base main --head feat/stk-orders
     --title "feat(stk-orders): buy/sell/cancel — Phase 2 write path (ADR 0017)"
     --body pointing at .pipeline/stk-orders artifacts + card; body MUST note the red-line
     docs amendment.
  7. Metadata commit on MAIN: card todo->review, current.json stage=impl + pr URL, journal
     seq=4, push main.
Feature gotchas (traps the next node MUST know):
  - Write calls ONLY in trade.rs (review greps place_order/submit_order/encode_place_order/
    cancel_order — zero hits elsewhere).
  - NO retry logic anywhere; timeout message MUST name the allocated order id and forbid
    blind retry.
  - Reuse TAKE_FIRST_TIMEOUT (pub in src/ib/mod.rs) — no new consts.
  - AGENTS.md/CLAUDE.md: ONLY the red-line paragraph per arch.md §Docs amendment.
  - NEVER touch tests/; never repo-wide cargo fmt; public repo: no secrets.
Done when: PR open, 16/16 + full suite + clippy green, freeze gate empty, card=review,
seq=4 pushed. On success: run pipeline-review (codex cli, YOLO).
On failure: attempts++; >=3 => blocked => run pipeline-hunt.
<<< END

## seq=4 · 2026-07-03T12:27:56Z · task→impl · completed · by=interactive-π/glm-5.2
done:   PHASE 2 WRITE PATH landed on feat/stk-orders (PR #15): cli.rs Buy/Sell/Cancel variants
        + OrderArgs/CancelArgs, NEW trade.rs (build_stk_order + shape_order_ack pure seams +
        require_live_write_gate effective-port double gate + buy/sell/cancel gateway fns with
        bounded first-ack loops reusing TAKE_FIRST_TIMEOUT), mod.rs re-exports, main.rs arms,
        AGENTS.md + CLAUDE.md red-line amendment (verbatim per arch §Docs). 6-file diff, +241 -3.
        Containment grep clean (write calls ONLY in trade.rs); no-retry grep clean; ordering
        invariant usage<config<connection; timeout envelope names allocated order id + forbids
        blind retry. Frozen spec untouched (freeze-gate diff empty). CLAUDE.md kept < 900 bytes
        (consolidated two redundant bullets into the single amendment — passes claude_md.rs).
output: PR #15 (https://github.com/jackypanster/oh-my-ib/pull/15), feat/stk-orders tip 8ae9527
--- handoff ---
>>> NEXT
Run pipeline-review on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=feat/stk-orders pr=https://github.com/jackypanster/oh-my-ib/pull/15
Model: frontier SOTA required — HIGHEST-STAKES review to date (first write path); operator assigns (this run: codex cli).
First: git pull --rebase; no .env in this repo.
Read for context (before acting):
  - AGENTS.md + CLAUDE.md — repo conventions (amended red-line: writes now gated, Phase 2)
  - .pipeline/stk-orders/tasks/01.md (status=review) — card scope + hard constraints + freeze coverage
  - .pipeline/stk-orders/arch.md — §Component design + §Docs amendment (verbatim impl source)
  - .pipeline/stk-orders/docs/adr/0017-write-path-safety.md — the safety architecture (binding)
  - .pipeline/stk-orders/PRD.md + CONTEXT.md — criteria 1-11 + glossary
  - src/ib/trade.rs — the ONLY write-call module (review surface)
  - PR #15 diff (gh pr diff 15) — full review surface
Your task (concrete, numbered):
  1. Freeze gate FIRST (deterministic): git diff 3692c71bc11e873b2be8f3c9448a2a8d4f4d9e8f <review-tip> -- tests/stk_orders_command.rs
     must be EMPTY. Non-empty ⇒ reject (attempts++, route impl; >=3 ⇒ hunt).
  2. Full-suite gate: checkout feat/stk-orders, run current.json.full-verify
     (["cargo build","cargo test"]) — must be GREEN (114/114 expected). Red attributable to
     card 01 ⇒ flip review→todo, attempts++; cross-card ⇒ reviews/integration-NN.md + hunt.
  3. Semantic review (by reading) of the PR #15 diff vs arch.md §Component design + card freeze
     coverage. THE POLARITY-FLIPPED CHECKS (write-path-specific, all required):
     a. CONTAINMENT: grep src/ for place_order/submit_order/encode_place_order/cancel_order —
        ZERO hits outside src/ib/trade.rs. No read command imports trade's gateway fns.
     b. DOUBLE GATE: require_live_write_gate gates on cfg.port == LIVE_PORT (effective port,
        catches --live AND hand-set --port 4001); OMI_ALLOW_LIVE=1 required; returns
        AppError::config (code=\"config\") BEFORE connect. Paper (:4002) ungated.
     c. ORDERING INVARIANT: in buy/sell (the place fn): local validation (usage) → gate (config)
        → connect (connection) → next_valid_order_id → build_stk_order → place_order → bounded
        first-ack. usage < config < connection (frozen tests depend on this).
     d. BOUNDED ACK: timeout_iter_data(TAKE_FIRST_TIMEOUT) + Instant-classified None; first
        OrderStatus OR OpenOrder is the ack; skip ExecutionData/CommissionReport; cancel uses a
        single .next() (CancelOrder has one variant). Timeout ⇒ AppError::timeout naming the
        ALLOCATED order id + \"may have been SUBMITTED\" + \"verify with omi orders\" +
        \"do NOT retry blindly\".
     e. NO RETRY: grep trade.rs for retry/re_place/re-submit — zero hits. A timeout is an
        UNKNOWN state, never a re-placement.
     f. PURE SEAMS: build_stk_order (exact ibapi Order fields: action/total_quantity/order_type
        LMT|MKR/limit_price/tif Day + Contract symbol/STK); shape_order_ack (exact 6-key object,
        MKT ⇒ limit_price null).
     g. DOCS AMENDMENT: AGENTS.md + CLAUDE.md red-line bullet replaced VERBATIM with arch.md
        §Docs amendment text. CLAUDE.md stays < 900 bytes (tests/claude_md.rs).
  4. READ-ONLY polarity preserved: reads unchanged (orders.rs/brief.rs/etc. untouched); the
     only write symbols anywhere are in trade.rs.
  5. If all pass: operator live acceptance on PAPER (PRD criterion 11, merge gate): far-LMT buy
     → omi orders shows working → cancel → omi completed-orders shows Cancelled → omi positions
     unchanged. Operator MUST log gateway into PAPER. Live trading is NEVER exercised by the
     pipeline; the double gate is the operator's own key. Then human-confirm + squash-merge.
  6. After human-confirm + merge: card 01 status review->done, current.json stage=done (drop pr?),
     append journal seq=5, push main.
Feature gotchas (project-specific traps the next node MUST know):
  - HIGHEST-STAKES review: this is the repo's first write path. The polarity flips — the normal
    \"no write code\" rule is REPLACED by \"write code ONLY in src/ib/trade.rs, unreachable from reads\".
  - Freeze gate is the deterministic two-commit diff over tests/stk_orders_command.rs — empty = pass.
  - The gate MUST precede connect (frozen tests assert usage < config < connection ordering).
  - Timeout envelope MUST name the allocated order id and forbid blind retry — this is the
    UNKNOWN-state posture (ADR 0017 §3-4). A timeout is NOT a re-placement trigger.
  - CancelOrder has only the OrderStatus variant (no events to skip) — cancel uses a single
    .next(), NOT a loop. clippy::never_loop enforced this.
  - CLAUDE.md < 900 bytes is frozen (tests/claude_md.rs); the amendment consolidated two
    redundant bullets into one to stay under.
  - Reuses TAKE_FIRST_TIMEOUT (pub in src/ib/mod.rs) — no new consts, no new error codes.
  - The metadata commit is already on MAIN (this entry rides it); the code diff is on feat/stk-orders.
  - Public repo: scan the PR diff for account ids/tokens/balances before merge.
Done when: PR #15 squash-merged after freeze gate empty + full-suite green + ALL polarity-flipped
semantic checks (containment/double-gate/ordering/bounded-ack/no-retry/seams/docs) pass +
operator PAPER live acceptance (criterion 11); card=done, journal seq=5 pushed.
On failure: attempts++; >=3 ⇒ blocked ⇒ run pipeline-hunt.
<<< END

## seq=5 · 2026-07-03T12:36:24Z · impl→impl · failed · by=claude-fable-5
done:   PAPER ACCEPTANCE FAILED (pre-review): omi buy hung >3min BEFORE any order reached the
        gateway (orders + completed-orders empty post-kill — safe kill, nothing placed).
        Root cause source-pinned: next_valid_order_id() = unbounded subscription.next()
        (orders/sync.rs:197) on a RequestIds shared request this gateway never answers (3rd
        dossier wedge; first PAPER sighting). ADR 0017's "bounded, crate-managed" claim was
        WRONG — arch gap, not impl (round 1 was verbatim). Routed: ADR 0018 (handshake-seeded
        local allocator client.next_order_id(), source-verified non-blocking), arch.md
        Amendment, card round 2 (attempts=1). Frozen spec unchanged.
output: docs/adr/0018-local-order-id-allocator.md, arch.md §Amendment, tasks/01.md Round 2
--- handoff ---
>>> NEXT
Run pipeline-impl ROUND 2 on the SAME branch (feat/stk-orders, PR #15 open).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=https://github.com/jackypanster/oh-my-ib/pull/15
Model: capable-local OK — operator assigns (interactive pi/omp).
First: git pull --rebase (main, get ADR 0018 + amendments); git checkout feat/stk-orders.
Your task (concrete, numbered):
  1. In src/ib/trade.rs replace the order-id allocation ONLY:
     client.next_valid_order_id()? -> client.next_order_id()  (per arch.md §Amendment).
  2. Verify: cargo build && cargo test --test stk_orders_command && cargo test && cargo
     clippy --all-targets -- -D warnings.
  3. Freeze gate: git diff 3692c71bc11e873b2be8f3c9448a2a8d4f4d9e8f HEAD --
     tests/stk_orders_command.rs must be EMPTY.
  4. Commit on branch ("fix(stk-orders): local handshake-seeded order-id allocator, ADR 0018"),
     push (updates PR #15).
  5. Metadata on MAIN: card todo->review (attempts stays 1), current.json stage=impl,
     journal seq=6, push.
Feature gotchas: ONE LINE change — nothing else; no retry logic; never touch tests/;
containment rule stands. Done when: PR updated, all green, gate empty, seq=6 pushed.
On success: run pipeline-review (codex cli). On failure: attempts++ (=2); >=3 => blocked => hunt.
<<< END

## seq=6 · 2026-07-03T12:38:32Z · impl→impl · completed · by=interactive-π/glm-5.2
done:   ROUND 2 one-line fix landed on feat/stk-orders (PR #15 updated): replaced
        client.next_valid_order_id()? (unbounded subscription.next() on RequestIds — this
        gateway's 3rd wedge, first PAPER sighting) with client.next_order_id() — the
        handshake-seeded local allocator (non-blocking, returns id_manager's next id, ADR 0018).
        Allocation line only; everything else (gate, ordering invariant, bounded ack, no-retry,
        UNKNOWN-state envelope naming the allocated id, containment) unchanged. 114/114 + clippy
        clean; frozen spec untouched (freeze-gate diff empty); containment + no-retry grep clean.
output: feat/stk-orders tip ee9291f (PR #15 updated), src/ib/trade.rs
--- handoff ---
>>> NEXT
Run pipeline-review on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=feat/stk-orders pr=https://github.com/jackypanster/oh-my-ib/pull/15
Model: frontier SOTA required — HIGHEST-STAKES review to date (first write path); operator assigns (this run: codex cli).
First: git pull --rebase; no .env in this repo.
Read for context (before acting):
  - AGENTS.md + CLAUDE.md — repo conventions (amended red-line: writes now gated, Phase 2)
  - .pipeline/stk-orders/tasks/01.md (status=review, attempts=1) — card scope + §Round 2
  - .pipeline/stk-orders/arch.md — §Component design + §Docs amendment + §Amendment (round 2: next_order_id)
  - .pipeline/stk-orders/docs/adr/0017-write-path-safety.md — the safety architecture (binding)
  - .pipeline/stk-orders/docs/adr/0018-local-order-id-allocator.md — the paper-wedge fix (amends 0017's allocation claim)
  - .pipeline/stk-orders/PRD.md + CONTEXT.md — criteria 1-11 + glossary
  - src/ib/trade.rs — the ONLY write-call module (review surface)
  - PR #15 diff (gh pr diff 15) — full review surface (round 1 + round 2 commits)
Your task (concrete, numbered):
  1. Freeze gate FIRST (deterministic): git diff 3692c71bc11e873b2be8f3c9448a2a8d4f4d9e8f <review-tip> -- tests/stk_orders_command.rs
     must be EMPTY. Non-empty ⇒ reject (attempts++, route impl; >=3 ⇒ hunt).
  2. Full-suite gate: checkout feat/stk-orders, run current.json.full-verify
     (["cargo build","cargo test"]) — must be GREEN (114/114 expected).
  3. Semantic review (by reading) of the FULL PR #15 diff (round 1 + round 2) vs arch.md
     §Component design + §Amendment + §Docs amendment + card freeze coverage. ALL polarity-
     flipped checks from seq=4 stand (containment/double-gate/ordering/bounded-ack/no-retry/
     seams/docs), PLUS the round-2 allocation fix:
     - Order id comes from client.next_order_id() (the handshake-seeded local allocator,
       non-blocking, returns id_manager's next id), NOT client.next_valid_order_id() (which was
       an unbounded subscription.next() that wedged on this gateway's RequestIds — ADR 0018).
     - The allocator returns i32 (not Result), so there is no fallible map_err on it.
     - The ordering invariant still holds: validation → gate → connect → allocate id →
       build → place → bounded first-ack. usage < config < connection.
  4. READ-ONLY polarity preserved: reads unchanged; write symbols ONLY in trade.rs.
  5. If all pass: operator live acceptance on PAPER (PRD criterion 11, merge gate): far-LMT buy
     → omi orders shows working → cancel → omi completed-orders shows Cancelled → omi positions
     unchanged. Operator MUST log gateway into PAPER. Then human-confirm + squash-merge.
  6. After human-confirm + merge: card 01 status review->done, current.json stage=done (drop pr?),
     append journal seq=7, push main.
Feature gotchas (project-specific traps the next node MUST know):
  - Round 2 swapped next_valid_order_id()? -> next_order_id() (ADR 0018). The two are NOT
    interchangeable: next_valid_order_id is an unbounded gateway round-trip (wedges on this
    gateway); next_order_id is the local handshake-seeded allocator (non-blocking). Do NOT flag
    the absence of next_valid_order_id or its map_err as a regression.
  - HIGHEST-STAKES review: first write path. The polarity flips — the normal \"no write code\" rule
    is REPLACED by \"write code ONLY in src/ib/trade.rs, unreachable from reads\".
  - Freeze gate is the deterministic two-commit diff over tests/stk_orders_command.rs — empty = pass.
  - The gate MUST precede connect (frozen tests assert usage < config < connection ordering).
  - Timeout envelope MUST name the allocated order id and forbid blind retry (UNKNOWN-state posture).
  - CancelOrder has only the OrderStatus variant — cancel uses a single .next(), NOT a loop.
  - CLAUDE.md < 900 bytes is frozen (tests/claude_md.rs).
  - Reuses TAKE_FIRST_TIMEOUT (pub in src/ib/mod.rs) — no new consts, no new error codes.
  - The metadata commit is already on MAIN (this entry rides it); the code diff is on feat/stk-orders.
  - Public repo: scan the PR diff for account ids/tokens/balances before merge.
Done when: PR #15 squash-merged after freeze gate empty + full-suite green + ALL polarity-flipped
semantic checks pass + operator PAPER live acceptance (criterion 11); card=done, journal seq=7 pushed.
On failure: attempts++ (=2); >=3 ⇒ blocked ⇒ run pipeline-hunt.
<<< END

## seq=7 · 2026-07-03T12:47:43Z · impl→review · completed · by=codex-gpt-5
done:   review verdict written: PR #15 APPROVED / merge-ready, NOT merged. Freeze gates empty,
        scope exactly 6 impl-paths, isolated full suite green (cargo build + cargo test 114/114 +
        clippy -D warnings) at ee9291f, semantic checks pass (containment, effective-port double
        gate, validation→gate→connect ordering, bounded UNKNOWN ack, no retry, R2 local
        next_order_id allocator, docs amendment). PAPER evidence recorded from /tmp/o1.json and
        /tmp/o3.json plus operator lifecycle report; operational note recorded: order-id cancel is
        client-id namespaced (cross-client-id cancel returns IB 10147).
output: .pipeline/stk-orders/reviews/review-01.md
--- handoff ---
>>> NEXT
Run pipeline-review merge continuation on a FRESH session after the human explicitly confirms merge.
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=https://github.com/jackypanster/oh-my-ib/pull/15
Model: frontier SOTA required — first write path; do not downgrade.
First: git pull --rebase; no .env in this repo.
Read for context:
  - AGENTS.md + CLAUDE.md
  - CONTRACT.md in jackypanster/pipeline
  - .pipeline/stk-orders/{PRD.md,arch.md,CONTEXT.md,tasks/01.md,journal.md}
  - .pipeline/stk-orders/docs/adr/0017-write-path-safety.md
  - .pipeline/stk-orders/docs/adr/0018-local-order-id-allocator.md
  - .pipeline/stk-orders/reviews/review-01.md
Your task:
  1. Confirm the human has explicitly authorized merge. Without that exact confirmation, STOP.
  2. Re-check PR #15 head is still ee9291f95535e6c579976b48eb6959c9f6436be7 or rerun the review gates
     against the new head if it moved.
  3. If unchanged and human-confirmed, squash-merge PR #15 via the GitHub adapter. Do not local-merge.
  4. After merge, on main: set card 01 status review->done, set current.json stage=done, append the
     final review->done journal entry, commit once, push main.
Feature gotchas:
  - Do not merge without explicit human confirmation; this seq=7 is approval evidence, not merge auth.
  - PR head reviewed was ee9291f95535e6c579976b48eb6959c9f6436be7.
  - R2 allocator is client.next_order_id(), not next_valid_order_id().
  - Order-id cancel is client-id namespaced; cross-client-id cancel returns structured IB 10147.
  - If PR head moved, rerun freeze gates, scope gate, full suite, clippy, and semantic checks before merge.
Done when: PR #15 squash-merged after human confirm, card=done, current.json stage=done, final journal
entry committed and pushed. On any regression: reject per CONTRACT (attempts++ to 2; >=3 => hunt).
<<< END
