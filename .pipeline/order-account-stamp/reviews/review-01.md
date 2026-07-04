# review-01 — order-account-stamp PR #21

Verdict: **APPROVE** — awaiting explicit human merge confirmation. No blocking findings.
This review did not merge PR #21.

Reviewer: claude (Claude, orchestrator acting as reviewer this round — π hit a 429 usage
cap mid-impl, so impl was handed to codex-gpt-5.5; review rotated to Claude. The
writer≠reviewer invariant holds: writers were π + codex, reviewer is a different model).

Review depth: deep. Reason: the PR changes the broker write path (account routing of
every placed order).

## Review surface

- PR: https://github.com/jackypanster/oh-my-ib/pull/21
- Base: `main` at `138e9bd` (trunk before the metadata commit)
- PR head: `217db672945d14d42409469d87ec8aa55f624555` (π's impl; codex verified it needed
  no follow-up product code and pushed it verbatim, preserving authorship)
- Spec-rev: `24a34dd0c10135eb77981b882e55a1eb4ff1d74c`
- PR diff: 3 files — `AGENTS.md`, `src/ib/mod.rs`, `src/ib/trade.rs` (27 insertions,
  6 deletions).

## Deterministic gates

- Freeze gate FIRST: PASS. `git diff 24a34dd origin/feat/order-account-stamp --` over ALL
  seven frozen spec files (`order_account_stamp`, `stk_orders`, `option_orders`,
  `option_combo`, `option_close`, `positions_row`, `close_pending_guard`) AND `CLAUDE.md`
  produced no output. `--name-status` confirms no frozen file modified.
- Scope gate: PASS. PR touches only the card's impl-paths (trade.rs, mod.rs, AGENTS.md).
- CLAUDE.md byte budget: PASS. Unchanged, 876 bytes (< 900).
- Dependency gate: PASS. No Cargo.toml/Cargo.lock change.

## Isolated verification

Detached worktree at PR head `217db67`:
- `cargo build`: pass.
- `cargo test` (FULL): pass, 220 tests (215 prior + 5 new `order_account_stamp`).
- `cargo clippy --all-targets -- -D warnings`: clean.
- Regression + card suites green within the full run.
- Cleanup: worktree removed.

## Semantic review

- **Seam** (`stamp_order_account`, trade.rs:252): one line, sets `order.account` only;
  matches the frozen matrix (overwrite + byte-identity). ✓
- **Choke point** (`place_with_client`, trade.rs:319): gains a REQUIRED `&AccountId`
  param; body `let mut order = order.clone(); stamp_order_account(&mut order, &account.0);`
  then the UNCHANGED allocate → `place_order(&order)` → bounded first-ack. The clone is
  the stamped order actually sent. ✓
- **Single sink**: exactly ONE `.place_order(` call site in the whole tree
  (trade.rs:339), on the stamped path — no placement can bypass the stamp. ✓
- **Call sites**: `place_core` (trade.rs:390) and `option_combo` (trade.rs:681) resolve
  the account AFTER connect and pass it; `option_close` (trade.rs:786) passes its
  ALREADY-resolved account — no duplicate `managed_accounts` (D4). Three resolve sites,
  one per function. ✓
- **Frozen builders untouched**: `build_stk_order`/`build_option_order`/`build_combo_order`
  not in the diff; their pure output (`account=""`) preserved — the stamp is gateway-path
  only. ✓
- **Authority**: `resolve_account` (cfg/`--account` else first managed) is the sole source;
  it errors when no account exists, so the stamp never receives an empty string in
  practice. ✓
- **AGENTS.md**: phrase added; CLAUDE.md untouched. ✓

## Adversarial pass

- **Bypass hunt**: grep for `.place_order(` across `src/` — single hit on the stamped path.
  No alternate placement route. Refuted.
- **Double-resolve on option_close**: grep confirms option_close has exactly one
  `resolve_account`; the stamp reuses it. Refuted.
- **Fragile assumption CONFIRMED-SAFE by live paper probe** (criterion 7, runnable today):
  `option-buy AAPL 20260918 240C --limit 0.05` on paper `:4002` (account DUQ653733) ⇒
  PreSubmitted (Tiger ACCEPTED an explicitly-stamped `Order.account` — no rejection);
  `omi orders` showed the working order carrying `account="DUQ653733"`; `omi cancel 4` ⇒
  Cancelled. This is the one load-bearing assumption in ADR 0024 §5 and it is REFUTED as a
  risk — Tiger accepts the explicit account; the fallback path is NOT needed.

## Residual risk

None blocking. Multi-account acceptance (a second live account routing test) remains
environmental/out-of-scope (single-account env); the fix makes routing EXPLICIT, which is
the intent. The prior features' deferred fill-lifecycle acceptance (option-close /
close-pending-guard) is unaffected by this PR.

## Disposition

APPROVE. Keep card 01 at `status: review`; do not mark done until PR #21 is human-confirmed
and squash-merged. If PR head moves from `217db67`, re-run the freeze gate, isolated
verification, semantic review, and the paper probe before merge.
