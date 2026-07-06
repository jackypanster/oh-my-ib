# review-01 — live-gate-test-safety card 01 (PR #26)

Verdict: ACCEPT + MERGED (squash `5b5b59b`). 2026-07-06. Reviewer: codex (GPT-5.5); merge gate + safety
proof + merge: cc; human-confirmed by operator.

## Deterministic gate (cc)
- Diff = `tests/stk_orders_command.rs` ONLY (zero `src/`, zero `Cargo.toml`/`Cargo.lock`).
- spec-paths empty (test-only fix, ADR 0029 §Freeze coverage) ⇒ no freeze-gate diff to run.

## Semantic review (codex, detached PR worktree 5601e54)
- Matches ADR 0029: `live_gateway_reachable()` = std `TcpStream::connect_timeout(127.0.0.1:4001, 300ms)`;
  guard at the TOP of `live_buy_with_env_passes_gate_and_fails_on_dead_gateway` — reachable ⇒ `return`
  BEFORE the `OMI_ALLOW_LIVE=1` + `buy --live` subprocess, so no order path is reached.
- `require_live_write_gate` untouched; the 4 gate-REJECT tests, paper dead-port test, pure-seam tests,
  and `omi()`/`expect_error_code` helpers all unchanged.
- `cargo test --test stk_orders_command` 16/16; `cargo build` OK; `cargo clippy --test stk_orders_command
  -- -D warnings` clean. (Full `--all-targets` intentionally NOT run — trunk-red from ①'s unmerged
  `option_chain_filter` frozen test; orthogonal.)

## Operational safety proof (cc, the real correctness gate — Tiger :4001 UP)
- `omi --live orders` before = 0 open orders.
- Ran the GUARDED `cargo test --test stk_orders_command` (gateway UP) — the dangerous test SKIPPED.
- `omi --live orders` after = 0 open orders. **ZERO orders placed** (vs. a real order before the fix).

## Cross-feature note
Merge order: ② (this) FIRST, then ① (PR #25) rebases onto trunk to absorb the guard, making ①'s
full-suite gate safe + its clippy green (once `filter_chain_rows` lands). Recorded for the ① resume.

## SKILL-PROPOSAL
pipeline (process): parking a feature mid-flight (its frozen red test on trunk) to start another creates
a trunk-red compile collision for the second feature's `--all-targets` gates — the "one feature in
flight" rule exists for exactly this. When a blocker-fix MUST interleave, document the trunk-red as
expected and gate the interleaved feature on card-scoped + operational proof, not `--all-targets`.
