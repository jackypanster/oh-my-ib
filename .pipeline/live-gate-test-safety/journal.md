# journal — live-gate-test-safety

## seq=1 · 2026-07-06T06:32:04Z · prd→arch · completed · by=claude-opus-4-8
done:   PRD written. A test places REAL live orders: tests/stk_orders_command.rs::live_buy_with_env_
        passes_gate_and_fails_on_dead_gateway runs `buy AAPL 1 --limit 1 --live --host 127.0.0.1` with
        OMI_ALLOW_LIVE=1, expecting a dead :4001; with the Tiger gateway UP it PLACES a real order (3
        found + cancelled live today). Goal: the test never places an order regardless of gateway state,
        still verifies gate-pass. Recommended: guard-skip if 127.0.0.1:4001 reachable (std-only), else
        assert `connection` as today. Test-only; no src change; the 4 gate-REJECT tests stay. This
        BLOCKS option-chain PR #25's full-suite merge gate. Re-freezes stk-orders' frozen spec file
        (orig spec-rev 3692c71) under this feature.
output: .pipeline/live-gate-test-safety/PRD.md
--- handoff ---
>>> NEXT
Run pipeline-arch on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
Model: frontier SOTA required — operator assigns the bot; the pipeline can't verify the model.
First: git pull --rebase; load repo config (.env if present, per CONTRACT step 2).
Read for context (before acting):
  - oh-my-ib/AGENTS.md + CLAUDE.md — repo conventions (agent-first; hard safety rules; Verify = 4 gates)
  - .pipeline/live-gate-test-safety/PRD.md — problem + recommended guard-skip decision + rejected alts
  - tests/stk_orders_command.rs — the target file: omi() helper (env_remove), the 4 SAFE gate-REJECT
    tests (91-114), the ONE DANGEROUS test live_buy_with_env_passes_gate_and_fails_on_dead_gateway (117),
    the SAFE paper dead-port test (128)
  - src/ib/trade.rs require_live_write_gate (175) — the gate (do NOT change it); src/config.rs LIVE_PORT
Your task (concrete, numbered):
  1. Confirm the mechanism (recommended: guard-skip via std::net::TcpStream::connect_timeout on
     127.0.0.1:4001 with a short timeout; reachable ⇒ eprintln + return; else assert `connection`).
     Consider a short timeout value (e.g. 300ms) so a genuinely-dead port fails fast in CI. Emit an ADR
     (next number 0029) recording the decision + the rejected alternatives (fast-fail loopback assumes
     bind addr; env-into-Config refactor = too much blast radius; #[ignore]/delete = drops coverage).
  2. Decide what the RE-FROZEN unit assertion is (a test can't safely assert "no order placed with a
     live gateway" — that is operator live acceptance). The frozen unit assertion is the dead-gateway
     branch (connection error) + the guard's presence; the safety is proven by operator live acceptance
     (criterion 1: `omi --live orders` empty before+after a full cargo test with the gateway UP).
  3. Write arch.md + CONTEXT.md (glossary: the double gate; gate-pass vs gate-reject; the guard) +
     docs/adr/0029-*.md. Do NOT write src or tests.
Feature gotchas (project-specific traps the next node MUST know):
  - Do NOT touch require_live_write_gate or any src/. Test-only fix.
  - The 4 gate-REJECT tests are ALREADY safe (env_remove; refused before connect) — leave them.
  - The gate keys on cfg.port == LIVE_PORT (4001); --live and --port 4001 both trigger it.
  - This modifies stk-orders' FROZEN test file; task re-freezes it under this feature with a NEW spec-rev.
  - Do NOT run full `cargo test` on this repo while the Tiger gateway is UP until the fix lands.
Done when: arch.md + CONTEXT.md + ADR 0029 exist; mechanism decided; current.json stage=arch; journal
seq=2 appended + pushed. On success: arch→task, run pipeline-task.
On failure: attempts++; >=3 ⇒ blocked ⇒ run pipeline-hunt.
<<< END

## seq=2 · 2026-07-06T06:35:52Z · arch→task · completed · by=claude-opus-4-8
done:   Arch + ADR 0029 accepted. Guard-skip: add std-only `live_gateway_reachable()`
        (TcpStream::connect_timeout 127.0.0.1:4001, 300ms) to tests/stk_orders_command.rs; guard the ONE
        dangerous test (reachable ⇒ eprintln+return; else assert `connection`). Test-only, no src.
        Pipeline handling: spec-paths EMPTY (can't-freeze case), impl-paths = the test file itself
        (coder-editable this feature; was frozen under DONE stk-orders 3692c71 — freeze gate only checks
        THIS feature's spec-paths = ∅). Safety proven by reviewed-by-reading + operator live acceptance
        (omi --live orders empty before+after a full cargo test with gateway UP).
output: .pipeline/live-gate-test-safety/{arch.md,CONTEXT.md,docs/adr/0029-live-gate-test-guard-skip.md}
--- handoff ---
>>> NEXT
Run pipeline-task on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
Model: frontier SOTA required — operator assigns the bot; the pipeline can't verify the model.
First: git pull --rebase.
Read: .pipeline/live-gate-test-safety/{PRD.md,arch.md,CONTEXT.md,docs/adr/0029-*}; tests/stk_orders_command.rs.
Your task (concrete, numbered):
  1. Author card 01 (single card, test-only). NO freeze commit — spec-paths is EMPTY (ADR 0029 §Freeze
     coverage: the meaningful test can't be frozen). Record card frontmatter:
     status: todo, attempts: 0, spec-paths: [] , impl-paths: [tests/stk_orders_command.rs],
     verify: [cargo build, cargo test --test stk_orders_command], spec-rev: "" (none — no freeze commit),
     and a `## Freeze coverage` note (NONE frozen; reviewed-by-reading + operator live acceptance).
  2. Advance current.json stage=task (full-verify stays [cargo build, cargo test]). Journal seq=3.
     Since there is no freeze commit, this is a SINGLE metadata commit (card + current.json + journal).
Feature gotchas:
  - Test-only fix; NO src. The freeze/red-test ceremony does NOT apply (nothing to freeze).
  - impl-paths is a TEST file — that is intentional (the fix edits a test). spec-paths ∩ impl-paths = ∅
    holds trivially (spec-paths empty).
  - Do NOT run full `cargo test` while the Tiger gateway is UP until the fix lands — card-scoped only.
Done when: card 01 exists (spec-paths empty); current.json stage=task; journal seq=3 pushed.
On success: task→impl, run pipeline-impl (assign π/OMP).
On failure: attempts++; >=3 ⇒ blocked ⇒ run pipeline-hunt.
<<< END

## seq=3 · 2026-07-06T06:37:03Z · task→impl · completed · by=claude-opus-4-8
done:   Card 01 recorded (single metadata commit — NO freeze commit; spec-paths EMPTY per ADR 0029).
        impl-paths = [tests/stk_orders_command.rs], verify = [cargo build, cargo test --test
        stk_orders_command] (card-scoped), freeze coverage = NONE (reviewed-by-reading + operator live
        acceptance). Guard-skip edit spelled out verbatim in the card.
output: .pipeline/live-gate-test-safety/tasks/01.md
--- handoff ---
>>> NEXT
Run pipeline-impl on a FRESH session — ASSIGN TO π / OMP.
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
Model: capable-local OK (impl only).
First: git pull --rebase.
Read: .pipeline/live-gate-test-safety/tasks/01.md (THE CARD, verbatim guard edit); arch.md; ADR 0029;
      tests/stk_orders_command.rs (the target; the dangerous test at line 117; the 4 SAFE reject tests).
Your task (concrete, numbered):
  1. git checkout -b feat/live-gate-test-safety (cut from trunk).
  2. Edit ONLY tests/stk_orders_command.rs per the card: add `use std::net::TcpStream; use
     std::time::Duration;`, add `live_gateway_reachable()`, guard the dangerous test (reachable ⇒
     eprintln + return; else the unchanged assertion). Touch NOTHING else — no src, no other tests.
  3. Green gates: cargo build / cargo test --test stk_orders_command (card-scoped) / cargo clippy
     --all-targets -- -D warnings. SAFETY: only run the stk_orders test AFTER your guard is in place
     (or confirm :4001 is down first) — the un-guarded test places a real live order.
  4. Open PR feat→main; set current.json.pr; journal seq=4 (impl→review); print pipeline-review handoff.
Feature gotchas:
  - Test-only. spec-paths is EMPTY — there is NO freeze gate to trip; just don't touch src or the other
    tests. The diff MUST be tests/stk_orders_command.rs only.
  - The gate/require_live_write_gate is UNCHANGED. The 4 gate-REJECT tests are UNCHANGED.
  - Shared worktree: you own it during impl; cc/codex won't run git here until your PR is up.
Done when: guard added, card-scoped test + clippy + build green, diff = the test file only, PR open,
current.json stage=impl + pr set, journal seq=4 pushed.
On success: impl→review, run pipeline-review (codex reads the diff; cc runs the SAFE full-suite gate +
operator live acceptance + merges).
On failure: attempts++; >=3 ⇒ blocked ⇒ run pipeline-hunt.
<<< END
