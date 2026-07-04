# review-01 - option-close PR #19

Verdict: APPROVE - awaiting operator paper acceptance / explicit merge confirmation. No
blocking findings. This review did not merge PR #19.

Review depth: deep. Reason: the PR adds a brokerage write path (`option-close`) that can
place orders, even though the diff itself is medium sized.

Review surface:
- PR: https://github.com/jackypanster/oh-my-ib/pull/19
- Base: `main` at `d150657b3640f5c34a9670d76f5830086abb7c78`
- PR head: `c7ae10ebd402f0a6fa6b8211ac8130e20a336936`
- Spec-rev: `286eb6ab0ea1728e8dfd32fe5be6a0443ae36016`
- PR diff: 7 files, 333 insertions, 9 deletions:
  `AGENTS.md`, `CLAUDE.md`, `src/cli.rs`, `src/ib/mod.rs`,
  `src/ib/positions.rs`, `src/ib/trade.rs`, `src/main.rs`.

## Deterministic gates

- Freeze gate FIRST: PASS. `git diff --exit-code 286eb6a c7ae10e --
  tests/positions_row.rs tests/option_close_command.rs` produced no output.
- Frozen spec name-status check: PASS. `git diff --name-status 286eb6a c7ae10e --
  tests/positions_row.rs tests/option_close_command.rs` produced no output.
- Test tree unchanged in PR diff: PASS. `git diff --name-only origin/main...refs/tmp/pr-19
  -- tests` produced no output.
- Scope gate: PASS. `gh pr diff 19 --name-only` returned only the union of the two cards'
  impl-paths.
- Dependency gate: PASS. `git diff --name-only origin/main...refs/tmp/pr-19 --
  Cargo.toml Cargo.lock` produced no output.
- Whitespace gate: PASS. `git diff --check origin/main...refs/tmp/pr-19` produced no
  output.
- Secret/account scan: PASS. The PR diff scan matched only the public repo warning text in
  AGENTS.md / CLAUDE.md; no credential or real account material was introduced.
- PR state: OPEN, MERGEABLE, head `c7ae10ebd402f0a6fa6b8211ac8130e20a336936`.

## Isolated verification

Detached worktree: `/tmp/oh-my-ib-option-close-review.MRe5bY` at PR head `c7ae10e`.

Commands run in that worktree:
- `cargo build`: pass.
- `cargo test`: pass, 199 tests passed plus doc-tests with 0 tests.
- `cargo clippy --all-targets -- -D warnings`: pass.
- `cargo test --test positions_row`: pass, 5 tests.
- `cargo test --test option_close_command`: pass, 21 tests.
- Cleanup: detached worktree removed after verification.

## Semantic review

Card 01, positions row:
- `src/ib/positions.rs:22` promotes `position_row` to `pub`, and
  `src/ib/mod.rs:39` re-exports it.
- `src/ib/positions.rs:27-47` implements the ADR 0022 null semantics: option identity fields
  are populated iff `SecurityType::Option`; non-OPT rows emit all four option identity fields
  as null; empty option multiplier is null.
- `src/ib/positions.rs:29-33` includes the non-exhaustive-safe right fallback arm.
- `src/ib/brief.rs` still consumes `super::positions::position_row`; no separate brief row
  shaper was introduced, so brief parity remains same-source.
- Read modules do not import trade gateway functions as part of this card.

Card 02, option-close:
- Validation ordering matches the freeze: `src/ib/trade.rs:697-728` performs usage checks
  before `require_live_write_gate` at `src/ib/trade.rs:731`, which is before the single
  `super::connect` at `src/ib/trade.rs:734`.
- Single-connect invariant holds. The one client from `src/ib/trade.rs:734` is reused for
  account resolution, `account_updates`, `contract_details`, and final placement.
- Anti-open gate is before any order placement: no matched row and zero position return
  `not_found` at `src/ib/trade.rs:759-777`; non-option rows return `usage` at
  `src/ib/trade.rs:779-788`.
- Anti-double gate holds: side and quantity come from `derive_close` at
  `src/ib/trade.rs:790-792`; no side CLI argument exists.
- Rebuild uses the proven builder chain at `src/ib/trade.rs:832-843`, with row
  `trading_class` iff non-empty and row currency or USD fallback.
- Wrong-contract gate is before placement: `contract_details` runs at
  `src/ib/trade.rs:847-855`, conid mismatch refuses at `src/ib/trade.rs:856-864`, and
  `place_with_client` is only reached at `src/ib/trade.rs:870`.
- Bounded first-ack/no-retry is inherited verbatim from `place_with_client`:
  `src/ib/trade.rs:291` allocates the order id, `src/ib/trade.rs:294-296` places once,
  `src/ib/trade.rs:302-329` takes the first `OrderStatus`/`OpenOrder` under
  `TAKE_FIRST_TIMEOUT`, and timeout text names the order id and forbids blind retry.
- Containment polarity holds for write calls: `rg -n "\.place_order\(|\.cancel_order\("
  src tests` hits only `src/ib/trade.rs:250` and `src/ib/trade.rs:295`.
- `contract_details` also exists in read commands by design; it is not the write containment
  sink. The close path still asserts its conid before placement.

CLI/docs:
- `src/cli.rs:95-96` adds the `OptionClose` command, and `src/cli.rs:250-261` defines
  only long flags: `--conid`, `--limit`, `--qty`.
- `src/main.rs:85` dispatches `Command::OptionClose` to `ib::option_close`.
- AGENTS.md names `option-close` in the Phase-2 option order list with close-by-conid /
  side-derived semantics.
- CLAUDE.md names `option-close` in the short option-orders line and is 876 bytes, under
  the frozen 900-byte budget.

## Adversarial pass

- Assumption violation: invalid local input cannot reach the gate or connection. Probe:
  `env -u OMI_ALLOW_LIVE ./target/debug/omi --format json option-close --conid 1 --limit 1
  --qty 1.5 --live` returned `code="usage"` with exit 64.
- Live-gate abuse: valid live input without `OMI_ALLOW_LIVE=1` fails closed before connect.
  Probe: `env -u OMI_ALLOW_LIVE ./target/debug/omi --format json option-close --conid 1
  --limit 1 --live` returned `code="config"` with exit 5.
- Connection path: valid paper input on a dead port reaches connection only after validation
  and gate. Probe: `env -u OMI_ALLOW_LIVE ./target/debug/omi --format json --host
  127.0.0.1 --port 65000 option-close --conid 1 --limit 1` returned `code="connection"`
  with exit 2.
- Cascade construction: stale/not-held conid and flat position are stopped by `not_found`
  before any contract rebuild or placement call.
- Composition failure: rebuilt contract identity drift is converted into `data`/`not_found`
  before `place_with_client`.

## Residual risk

PRD criterion 12 paper lifecycle acceptance was not run in this review. That requires a real
paper option position and gateway state. Merge continuation should require operator paper
acceptance or an explicit operator waiver before squash-merge.

## Disposition

APPROVE. Keep both option-close cards at `status: review`; do not mark done until PR #19 is
human-confirmed and squash-merged. If PR head moves from `c7ae10e`, rerun the freeze gate,
isolated verification, semantic review, and adversarial probes before merge.
