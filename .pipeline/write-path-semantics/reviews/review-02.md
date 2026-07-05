# Review 02 — write-path-semantics

Verdict: changes-requested. Do not merge PR #22 yet.

## Review Surface

- PR: https://github.com/jackypanster/oh-my-ib/pull/22
- Base: `main`
- Head: `feat/write-path-semantics` / `013e84d7f4c0a704f781016331d0bdaaeb73a765`
- Net diff: `A docs/write-path-semantics.md`; no `src/` diff and no frozen spec diff.
- Freeze gate: PASS. `git diff 1549375611870e690e1a6717df63de515b5691d5 013e84d7f4c0a704f781016331d0bdaaeb73a765 -- tests/write_path_semantics_doc.rs` produced empty output.
- Full verify on detached PR-head worktree: PASS. `cargo build`; `cargo test`.
- Review-01 regression checks: PASS for command shape. No `--paper` remains in the PR-head doc, expiry examples are `YYYYMMDD`, and the three probe commands parse through to a dead-port `connection` envelope when run with `--host 127.0.0.1 --port 65000`.

## Findings

1. `docs/write-path-semantics.md:87` / `docs/write-path-semantics.md:88` — the "SELL-credit per IBKR" probe uses the same `SELL lower / BUY higher` leg vector while changing only the whole-order action to `sell`, so it does not test the stated credit-spread shape.

   Trigger: an operator runs the first combo probe:
   `omi option-combo --action sell --leg "SELL 1 AAPL 20260918 240 C" --leg "BUY 1 AAPL 20260918 250 C" --qty 1 --limit 0.05 ...`.

   In this code path, `build_combo_order` stores the whole-order side in `order.action` and stores each DSL leg action separately in `contract.combo_legs`. IBKR's TWS API combo lesson states that the whole combo action multiplies the leg actions: selling the whole combo inverts the effective legs. Therefore `--action sell` with `SELL lower / BUY higher` is effectively the inverse of the documented bought credit spread; it is not the "SELL-credit" control the row claims. The existing frozen combo test's only negative-credit example is `Action::Buy` with `SELL / BUY` legs and a negative limit, which matches the IBKR guide's "buy a credit spread" example. The current recipe would send a live/paper probe that cannot disambiguate Tiger's sign convention because it is probing a different effective spread than the prose says.

   Required fix: document the scalar-vector interaction explicitly: `Order.action` is the whole-combo side and leg actions are multiplied by it. Then make the probe pair structurally coherent. For example:
   - BUY credit spread: `--action buy` + `SELL lower / BUY higher` + negative limit.
   - SELL spread for credit: use the opposite leg vector (`BUY lower / SELL higher`) with `--action sell` + positive limit.
   Keep the `⚠️` Tiger probe if still unverified, but the recipes must name the effective position they actually create.

## Evidence

- PR-head doc re-read with line numbers: `docs/write-path-semantics.md:45` now states action-relative pricing; `:67`, `:88`, `:92` use valid `omi ...` commands with `20260918`.
- CLI source re-read: global `--port` default is paper at `cli.rs:28`; `--live` selects live at `cli.rs:31`; `PAPER_PORT = 4002` at `config.rs:12`.
- PR-head command probes with `--host 127.0.0.1 --port 65000`: option-buy recipe and both option-combo recipes all reached `code="connection"`, so parse/usage shape is now valid.
- PR-head code re-read: `build_combo_order` writes `order.action = side` and each `ComboLeg.action` from the leg DSL (`src/ib/trade.rs:561-577`).
- Frozen test re-read: `negative_net_limit_is_a_credit_and_builds` uses `Action::Buy` + `SELL/BUY` legs + negative limit (`tests/option_combo_command.rs:125-135`), not `Action::Sell`.
- Official IBKR docs checked: "Notes on Combination Orders" defines combo limit signs by buy/sell spread cashflow; "TWS Python API Placing Complex Orders" explains that selling a combo multiplies/inverts the leg actions.

## Sign-off

files changed:    1 net PR file (+107 -0)
scope:            on target, but incomplete due incorrect combo probe semantics
review depth:     standard (doc-only diff over high-stakes write-path semantics)
hard stops:       1 found, 0 fixed by review
specialists:      none
new tests:        1 frozen doc test already present; PR makes 4/4 green
doc debt:         none outside this PR doc
verification:     freeze gate -> pass; cargo build -> pass; cargo test -> pass; semantic review -> fail
