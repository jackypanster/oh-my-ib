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
