# review-01 — pnl-by-position card 01, PR #9 @ 308891b

Verdict: **APPROVE — awaiting explicit human confirm + PRD D3 live acceptance.**

## Deterministic gates (all ran this session, in order)

1. **Freeze gate** — `git diff 367d671..308891b -- tests/pnl_by_position_command.rs` → **EMPTY** ✓
   (two-commit diff against the PR head sha, not the working tree).
2. **Full-suite gate** — detached worktree at 308891b: `cargo build` ✓, `cargo test` → **12 test
   targets, 56 cases, 0 failed** ✓ (trunk red resolved by this head, as designed).
3. **Clippy** — `cargo clippy --all-targets -- -D warnings` on the head → clean ✓.
4. `spec-paths ∩ impl-paths = ∅` re-checked ✓ (diff touches only the 4 impl-paths files).

## Semantic review (gateway fn read by hand per card Freeze coverage)

Reviewed by the session + an independent adversarial subagent (fresh context, ibapi-3.1.0 source as
ground truth). All 7 binding constraints verified:

- take-first per `pnl_single` (`next_data()` = first item, notices filtered — ibapi
  `subscriptions/sync.rs:242`); no drain loop anywhere.
- fail-fast on `Some(Err)`/`None` naming the conid; `Ok` built only after the full sweep — no
  partial `by_position` possible; errors exit via the stderr JSON envelope.
- symbol from discovery; position/value/PnL from the reading.
- all 4 money fields through `pnl_number`; bare-f64 sentinel path live; non-finite cannot produce
  malformed JSON.
- qty==0 rows unfiltered (frozen test pins this).
- `drop(subscription)` before the sweep **actually cancels** (verified: `snapshot_ended`
  early-return can't suppress it — neither decoder overrides `is_snapshot_end`; shared-channel
  cancel fires `encode_cancel_account_updates`; each `pnl_single` drop fires
  `encode_cancel_pnl_single(id)`, including on early-error returns — no leaked subscriptions).
- `ContractId(pub i32)` + `From<i32>`; `Contract.symbol` is `Symbol(pub String)` → `.to_string()`
  correct; error-context style matches pnl.rs/positions.rs; seam split mirrors executions.rs.

## Findings

- ADVISORY `src/ib/pnl_by_position.rs:60` — duplicate conid rows possible if a portfolio update
  lands mid-download; same undeduped behavior as the accepted `positions.rs`; output stays
  well-formed. No action.
- ADVISORY `src/ib/pnl_by_position.rs:89` — a wedged gateway that acks reqPnLSingle but never
  emits data/error hangs the read; this is exactly ADR 0009 decision 4 (blocking default,
  `next_timeout` recorded fallback if live acceptance shows hangs). No action now.

Zero CRITICAL/HIGH/MEDIUM/LOW. Scope on target: every hunk traces to card 01.

## HARD GATE before merge (PRD D3 — not yet satisfied)

Operator must, in order, against the live Tiger gateway (`:4001`):
1. `omi --live pnl` → numeric `daily_pnl`, no `1.7e308` leak (proves reqPnL family support).
2. `omi --live pnl-by-position` → rows for held positions (or `[]` flat); observe whether qty==0
   closed-today rows appear (D6 live observation).
3. Explicitly authorize the merge.

Gateway state at review time: CLOSED (`:4001` connection refused). Merge is BLOCKED on the
operator; only pipeline-review merges, human-confirmed.
