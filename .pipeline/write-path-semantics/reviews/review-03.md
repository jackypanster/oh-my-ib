# Review 03 — write-path-semantics

Verdict: approved. PR #22 is ready for human-confirmed merge.

## Review Surface

- PR: https://github.com/jackypanster/oh-my-ib/pull/22
- Base: `main`
- Head: `feat/write-path-semantics` / `846e719665d2398a62726523ea8f3c4e55317891`
- Net diff: `A docs/write-path-semantics.md`; no `src/` diff and no frozen spec diff.
- Freeze gate: PASS. `git diff 1549375611870e690e1a6717df63de515b5691d5 FETCH_HEAD -- tests/write_path_semantics_doc.rs` produced empty output.
- Full verify on detached PR-head worktree: PASS. `cargo build`; `cargo test`.
- Review-01 regression checks: PASS. No `--paper`; probe expiries use `YYYYMMDD`; recipes use default paper invocation shape.
- Review-02 regression checks: PASS. The combo probe triplet now uses coherent `Order.action` / leg-vector / sign triples under the IBKR scalar-vector model.

## Findings

None.

## Evidence

- PR metadata: `gh pr view 22` reported open, non-draft PR #22, base `main`, head `feat/write-path-semantics`, head OID `846e719665d2398a62726523ea8f3c4e55317891`, 4 commits.
- Scope: `git diff --name-status main...FETCH_HEAD` showed only `A docs/write-path-semantics.md`; `git diff --check main...FETCH_HEAD` was clean.
- PR-head doc re-read with line numbers:
  - `docs/write-path-semantics.md:26` documents combo scalar-vector behavior on the `action` row.
  - `docs/write-path-semantics.md:45` documents IBKR action-relative combo sign plus scalar-vector leg inversion.
  - `docs/write-path-semantics.md:87-104` contains the three coherent paper probe commands and cleanup step.
- Local source re-read:
  - `src/cli.rs:28-33` has paper as the default port shape and `--live`, with no `--paper`.
  - `src/cli.rs:236-237` has the current simplified combo help string and `allow_hyphen_values`.
  - `src/ib/trade.rs:563-577` stores each `ComboLeg.action` from the DSL and passes `Order.action` plus `limit_price` independently.
  - `src/ib/trade.rs:317-329` stamps the resolved account at the placement choke point.
  - `tests/option_combo_command.rs:125-135` still pins the build-level negative-credit shape as `Action::Buy` with `SELL/BUY` legs.
- ibapi source re-read:
  - `orders/mod.rs:478-624` custom `Order::default()` pins the load-bearing defaults documented in the table.
  - `contracts/builders.rs` pins SMART/USD, option multiplier 100, and Spread/BAG combo leg construction.
- Official IBKR docs checked:
  - Trader Workstation "Notes on Combination Orders" defines spread limit signs relative to buy/sell spread cashflow.
  - IBKR Campus "TWS Python API Placing Complex Orders" explains combo buy/sell and leg actions as vector-scalar multiplication.
- Recipe shape probes on detached PR-head worktree with `--host 127.0.0.1 --port 65000`:
  - display-size option-buy recipe reached `code="connection"`.
  - BUY-credit combo recipe reached `code="connection"`.
  - SELL-credit combo recipe reached `code="connection"`.
  - divergence combo recipe reached `code="connection"`.

## Sign-off

files changed:    1 net PR file (+121 -0)
scope:            on target
review depth:     standard (doc-only diff over high-stakes write-path semantics)
hard stops:       0 found, 0 fixed, 0 deferred
specialists:      none
new tests:        1 frozen doc test already present; PR makes 4/4 green
doc debt:         none
verification:     freeze gate -> pass; cargo build -> pass; cargo test -> pass; semantic review -> pass
