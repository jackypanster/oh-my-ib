# Review 01 — write-path-semantics

Verdict: changes-requested. Do not merge PR #22 yet.

## Review Surface

- PR: https://github.com/jackypanster/oh-my-ib/pull/22
- Base: `main`
- Head: `feat/write-path-semantics` / `5364482caea3b5e4bc20d392340bcf8e03b43912`
- Net diff: `A docs/write-path-semantics.md`; no `src/` diff and no frozen spec diff.
- Freeze gate: PASS. `git diff 1549375611870e690e1a6717df63de515b5691d5 5364482caea3b5e4bc20d392340bcf8e03b43912 -- tests/write_path_semantics_doc.rs` produced empty output.
- Full verify on detached PR-head worktree: PASS. `cargo build`; `cargo test`.

## Findings

1. `docs/write-path-semantics.md:67` / `docs/write-path-semantics.md:82` — risk-register recipes use `omi --paper`, but the CLI has no `--paper` flag.

   Trigger: an operator copies either deferred probe recipe exactly. Current `omi --help` exposes paper as the default port and `--live` for live, but no `--paper` flag. A direct probe of the command surface returned exit code 64 with `{"error":{"code":"usage","context":"command-line arguments","message":"unexpected argument '--paper' found"}}`. The card requires runnable `:4002` probe recipes; these fail before connecting to the paper gateway.

   Required fix: replace every `omi --paper ...` recipe with the actual paper invocation shape, either plain `omi ...` (default `:4002`) or `omi --port 4002 ...` if the recipe wants to be explicit. Update the surrounding text so the recipe is copy-paste runnable.

2. `docs/write-path-semantics.md:45` / `docs/write-path-semantics.md:82` — the combo credit-sign row and probe flatten IBKR's action-relative combination pricing into "negative = credit".

   Trigger: the recipe tests a SELL credit spread with `--action sell ... --limit -0.05`. IBKR's TWS combination-order guide defines the sign relative to whether the spread is bought or sold: buying a credit spread uses a negative limit, but selling a spread and receiving cash uses a positive limit; selling a spread and owing cash uses a negative limit. The current row says "negative = credit, positive = debit" and the recipe uses negative with `--action sell`, so the reference-semantics column is not true as written.

   Required fix: revise the row to state the IBKR/TWS action-relative convention, then keep any Tiger-specific uncertainty as `⚠️ UNVERIFIED`. Adjust the probe recipe to match the semantic being tested: use `--action buy` for a negative-credit spread, or use `--action sell --limit 0.05` for a sell-credit spread. If the goal is to test whether `omi`'s existing `--action sell --limit -0.05` behavior diverges from IBKR docs, say that explicitly as a suspected divergence, not as the reference semantics.

## Evidence

- `gh pr view 22 --json ...`: PR is open, non-draft, base `main`, head `feat/write-path-semantics`, head OID `5364482caea3b5e4bc20d392340bcf8e03b43912`, merge state `CLEAN`.
- `git diff --name-status main...5364482caea3b5e4bc20d392340bcf8e03b43912`: only `A docs/write-path-semantics.md`.
- `target/debug/omi --paper orders`: reproduces the non-runnable recipe; exits usage with unexpected `--paper`.
- Local ibapi source checked: `orders/mod.rs` pins `Order::default()` load-bearing values (`transmit`, `display_size`, `outside_rth`, `origin`, `exempt_code`, `what_if`) and `contracts/builders.rs` pins SMART/USD/multiplier defaults.
- Official IBKR/TWS sources checked: Order class reference for `Transmit`; Basic Contracts for STK/OPT contract fields; Trader Workstation "Notes on Combination Orders" for action-relative combo pricing.

## Sign-off

files changed:    1 net PR file (+99 -0)
scope:            on target, but incomplete due non-runnable probes and incorrect combo reference semantics
review depth:     standard (doc-only diff over high-stakes write-path semantics)
hard stops:       2 found, 0 fixed by review (review cannot author product/doc fixes on the PR)
specialists:      none
new tests:        1 frozen doc test already present; PR makes 4/4 green
doc debt:         none outside this PR doc
verification:     freeze gate -> pass; cargo build -> pass; cargo test -> pass; semantic review -> fail
