# review-01 - close-pending-guard PR #20

Verdict: APPROVE - awaiting explicit human merge confirmation. No blocking findings.
This review did not merge PR #20.

Review depth: deep. Reason: the PR changes a broker write-path guard that can allow or
refuse order placement.

Review surface:
- PR: https://github.com/jackypanster/oh-my-ib/pull/20
- Base: `main` at `272dee56b50905c275b5f16dd4e5e7ea0482e701`
  (local `origin/main` during review)
- PR head: `b4b67c78d9761faba7031e0b948d4c0bab8d47de`
- Spec-rev: short `61f17e8`, resolved locally to
  `61f17e82125c25e56ab68713b7003b89d1e69e21`
- PR diff: `AGENTS.md`, `src/ib/mod.rs`, `src/ib/trade.rs` (3 files, 85 insertions,
  2 deletions).

Process note: `.pipeline/close-pending-guard/tasks/01.md` currently records the expanded
spec-rev as `61f17e8443b48083ec92a1a985ad06a35ebc7797`, which is not a local object.
The user prompt and journal both name the short spec-rev `61f17e8`; that short hash is
unambiguous and was the object used for deterministic freeze gates. I did not edit the card
frontmatter because review's write-set does not include spec-rev metadata repair.

## Deterministic gates

- Freeze gate FIRST: PASS. `git diff --exit-code 61f17e82125c25e56ab68713b7003b89d1e69e21
  b4b67c78d9761faba7031e0b948d4c0bab8d47de -- tests/close_pending_guard.rs`
  produced no output.
- Older frozen specs untouched in PR diff: PASS. `git diff --name-only
  origin/main...refs/tmp/pr-20 -- tests/option_close_command.rs tests/positions_row.rs`
  produced no output.
- Wider frozen-spec check: PASS. `git diff --exit-code 61f17e82125c25e56ab68713b7003b89d1e69e21
  b4b67c78d9761faba7031e0b948d4c0bab8d47de -- tests/option_close_command.rs
  tests/positions_row.rs` produced no output.
- CLAUDE.md untouched: PASS. `git diff --exit-code 61f17e82125c25e56ab68713b7003b89d1e69e21
  b4b67c78d9761faba7031e0b948d4c0bab8d47de -- CLAUDE.md` produced no output.
- Scope gate: PASS. `gh pr diff 20 --name-only` returned only `AGENTS.md`,
  `src/ib/mod.rs`, `src/ib/trade.rs`.
- Dependency gate: PASS. `git diff --name-only origin/main...refs/tmp/pr-20 --
  Cargo.toml Cargo.lock` produced no output.
- Whitespace gate: PASS. `git diff --check origin/main...refs/tmp/pr-20` produced no output.
- Secret/account scan: PASS. The only hit was the existing AGENTS.md public-repo warning text.
- Forge state: PR #20 open, mergeable, head `b4b67c78d9761faba7031e0b948d4c0bab8d47de`;
  CodeRabbit check passed.

## Isolated verification

Detached worktree: `/tmp/oh-my-ib-close-pending-review.Ay6pNZ` at PR head `b4b67c7`.

Commands run in that worktree:
- `cargo build`: pass.
- `cargo test`: pass, 215 tests plus doc-tests with 0 tests.
- `cargo clippy --all-targets -- -D warnings`: pass.
- `cargo test --test close_pending_guard`: pass, 8 tests.
- Cleanup: detached worktree removed after verification.

## Semantic review

Card 01 seam:
- `src/ib/trade.rs:227-246` implements `blocking_close_order_ids(position, conid,
  &[(order_id, conid, action)]) -> Vec<i32>`.
- Long positions block exact action `"Sell"` on same conid; short positions block exact
  action `"Buy"` on same conid; zero position returns empty; IDs are sorted ascending.
- `tests/close_pending_guard.rs:27-79` covers long/short block and no-block cases,
  other-conid, multiple blockers ascending, zero-position totality, and empty book.
- `src/ib/mod.rs:45` re-exports `blocking_close_order_ids`.

Guard wiring:
- Placement is correct: `option_close` validates and drains the held position, rejects not-held
  or flat rows, rejects non-OPT at `src/ib/trade.rs:806-816`, then runs the pending-close
  guard at `src/ib/trade.rs:818-872`, before `derive_close` at `src/ib/trade.rs:873-875`.
- Single-connect invariant holds in the option_close range: one `super::connect(cfg)` at
  `src/ib/trade.rs:762`; the guard reuses the same client via
  `super::orders::open_orders_with_client(&client, None, ctx)` at `src/ib/trade.rs:822`.
- Empty open-order drain is distinct from malformed data. `open_orders_with_client` returns
  `Value::Array(out)` at `src/ib/orders.rs:50`; an empty array becomes empty triples and no
  blocker. A non-array result or any row missing/invalid `order_id`, `conid`, or `action`
  returns `AppError::data` at `src/ib/trade.rs:823-853`; there is no skip-and-continue path.
- Refusal is `not_found` at `src/ib/trade.rs:863-871`; message includes the target conid,
  every blocking ID via the sorted `[ids]` list, `omi cancel <id>`, `omi orders`, and the
  "second close would flip the position" warning.
- `derive_close` and `shape_option_close_ack` signatures are unchanged by the PR diff; the
  only new public trade function is `blocking_close_order_ids`.

Containment/docs:
- Write-call containment holds: `rg -n "\.place_order\(|\.cancel_order\(" src tests`
  hits only `src/ib/trade.rs:278` and `src/ib/trade.rs:323`.
- AGENTS.md Phase-2 option-close phrase now says it refuses while a working close order
  exists on the conid.
- CLAUDE.md is untouched and remains 876 bytes.

## Adversarial pass

- Debug string case assumption: the seam is intentionally exact-case `"Buy"`/`"Sell"`.
  Runtime input comes from `src/ib/orders.rs:40` using `format!("{:?}", d.order.action)`.
  The pinned `ibapi 3.1.0` `Action` enum derives `Debug` and has variants `Buy` and `Sell`
  (`ibapi-3.1.0/src/orders/mod.rs:649-655`), so the current drain and seam agree. A future
  dependency change to this representation would need a new freeze.
- Empty array vs missing fields: attacked separately. Empty book is frozen by
  `tests/close_pending_guard.rs:76-79` and returns no blockers. Missing or invalid row
  fields return `data` errors at `src/ib/trade.rs:836-853`; a malformed row cannot be
  silently skipped.
- Cascade construction: retry-after-timeout is now blocked if the first close reached
  `all_open_orders`; if no order reached the gateway, the open-orders array is empty and
  the close may proceed.
- Residual TOCTOU is accepted by ADR 0023; no new issue found beyond the documented
  scan-then-place non-atomicity.

## Residual risk

PRD criterion 7 paper lifecycle is explicitly deferred to the next US trading session and is
combined with option-close's paper lifecycle. Offline gates plus semantic review are the merge
basis for this PR per the handoff.

## Disposition

APPROVE. Keep card 01 at `status: review`; do not mark done until PR #20 is human-confirmed
and squash-merged. If PR head moves from `b4b67c78d9761faba7031e0b948d4c0bab8d47de`, rerun
the freeze gates, isolated verification, semantic review, and adversarial pass.
