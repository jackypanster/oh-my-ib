# review-01 — option-chain-default-exchange card 01 (PR #25)

Verdict: ACCEPT + MERGED (squash `550619e`). 2026-07-06. Semantic review: codex (GPT-5.5); freeze gate +
full-suite gate + live acceptance + merge: cc; human-confirmed by operator. Blocked-then-unblocked by
live-gate-test-safety (PR #26) — see journal seq=6 (HELD) / seq=7.

## Deterministic freeze gate (cc) — PASS
`git diff 620362c HEAD -- tests/option_chain_filter.rs` = EMPTY (spec-paths untouched, incl. after the
rebase onto the guard). spec-rev on card = 620362c (the clippy-clean re-freeze).

## Semantic review (codex, detached PR worktree) — ACCEPT
Matches ADR 0028: `filter_chain_rows` pure (`""` passthrough; else exact-string case-sensitive retain,
no dedup; no-match ⇒ honest `chains:[]`); reqSecDefOptParams called with `""` (unfiltered); seam inserted
after the drain, before the UNTOUCHED `shape_option_chain`; re-export present; cli help + doc comments
updated. Read-path polarity clean — `trade.rs` / `require_live_write_gate` / `OMI_ALLOW_LIVE` untouched;
`ChainRow` / `shape_option_chain` / conid / timeout-drain unchanged.

## Full-suite gate (cc, post-rebase onto the guard) — PASS
`cargo test` = 242 tests / 30 suites, 0 failed. `cargo clippy --all-targets -- -D warnings` clean. Safe
with the Tiger gateway UP (the live-gate-test-safety guard skips the dangerous stk test) — `omi --live
orders` = 0 after the full run.

## Operator live acceptance (Tiger :4001, PRD criteria 1-3) — PASS
1. `omi --live option-chain AAPL` (no flag) ⇒ exactly the SMART row (25 expiries) — was `chains:[]` before.
2. `omi --live option-chain AAPL --exchange ""` ⇒ all 20 rows (incl SMART).
3. `omi --live option-chain AAPL --exchange AMEX` ⇒ only the AMEX row.

## Arc
prd → arch (ADR 0028) → task (freeze bb7336a) → impl (PR #25) → RE-FREEZE (clippy doc_lazy_continuation
in the frozen spec → 620362c) → impl rebased → codex ACCEPT → HELD (full-suite gate unsafe: stk live
test placed real orders) → live-gate-test-safety fix merged → rebased + gated + live-accepted → MERGED.
