# review-01 - search-command (PR #13, card 01)

Verdict: **APPROVE** - awaiting explicit operator merge confirmation. No blocking findings.
All deterministic gates are green, the implementation matches arch.md section "Component design",
and operator live acceptance evidence satisfies PRD criterion 8.

Reviewed head: `a4d6fa87baedc89fb1ef427372a7816d94e59f8b` (`feat/search-command`, PR #13).
Spec-rev: `db074c66d37f0dce8544cd9a84e6dadbf33f976d`.

## Deterministic gates

- **Freeze gate: EMPTY.** `git diff db074c66d37f0dce8544cd9a84e6dadbf33f976d a4d6fa87baedc89fb1ef427372a7816d94e59f8b -- tests/search_command.rs` produced no output.
- **Whole tests tree untouched:** `git diff main a4d6fa8 -- tests/` produced no output.
- **Scope exact:** `git diff --stat main...a4d6fa8` is exactly the card's four impl-paths:
  `src/cli.rs`, `src/ib/search.rs`, `src/ib/mod.rs`, `src/main.rs` (4 files, +87).
- **Whitespace check:** `git diff --check main...a4d6fa8` produced no output.
- **PR head confirmed:** `gh pr view 13 --json headRefOid,baseRefName,headRefName,url,title`
  returned head `a4d6fa87baedc89fb1ef427372a7816d94e59f8b`, base `main`, head branch
  `feat/search-command`.
- **Full-suite gate: GREEN** on detached worktree `/tmp/codex-review-wt` at `a4d6fa8`:
  `cargo build` passed; `cargo test` passed (89 passed, 0 failed); `cargo clippy --all-targets -- -D warnings` passed.

## Semantic review

Direct review surface: `git diff main...a4d6fa8` plus direct `git show a4d6fa8:<path>` reads.

- `src/cli.rs`: adds only `Command::Search(SearchArgs)` and required positional
  `SearchArgs.pattern: String`; no sec-type/exchange/currency filter surface added.
- `src/ib/search.rs`: matches arch.md. It connects once, calls
  `client.matching_symbols(&args.pattern)` exactly once, maps errors to
  `AppError::data(..., "search")`, then shapes the returned vector.
- Field mapping is exact: `conid`, `symbol`, `sec_type`, `primary_exchange`, `currency`,
  `description`, `derivative_sec_types`. Newtype fields render via `.to_string()`;
  strings pass through, including empty description.
- `shape_search` emits a JSON array in input/gateway order, with exactly the frozen 7-key row
  shape. Empty input naturally returns `[]`.
- Review-must-read forbidden surfaces are absent from the gateway function: no STK guard, no
  account resolution, no md-type switch, no `TAKE_FIRST_TIMEOUT`, no drain loop, no sorting.
  A targeted grep found only existing global declarations/comments for those names, not search
  behavior.
- `src/ib/mod.rs` only adds `mod search;` and re-exports `search`, `shape_search`, `SearchRow`.
- `src/main.rs` only adds the dispatch arm `Command::Search(args) => ib::search(&config, args)`.
- `output.rs` and `error.rs` are untouched; table rendering and error envelope behavior remain
  generic.
- Secret scan of the PR diff for account ids, tokens, secrets, balances produced no matches.

## CLI command surface

- Entrypoint remains `omi`; this PR adds the `search` subcommand only.
- Frozen tests cover help listing, `search --help`, missing pattern -> `usage` envelope, and
  dead gateway -> `connection` envelope.
- stdout/stderr contract is unchanged: success through existing JSON/table output layer, errors
  through the existing structured envelope.
- The command is read-only and non-mutating; no safety sink, filesystem mutation, order
  placement, account lookup, or credential surface is introduced.

## Live acceptance evidence (operator-run, PR head binary, live :4001)

I read the operator-provided JSON files in `/tmp`:

- `/tmp/s1.json`: `jq length` = 31; contains an AAPL row with `conid` 265598,
  `primary_exchange` NASDAQ, `currency` USD, `sec_type` STK, description `APPLE INC`.
  Operator reported command: `omi --live search apple`, exit 0, 1.4s.
- `/tmp/s2.json`: `jq length` = 17; first row is Tencent: symbol `700`,
  `primary_exchange` SEHK, `currency` HKD, description `TENCENT HOLDINGS LTD`.

Note: `wc -l` reports 300 and 173 physical lines respectively because the files are pretty JSON;
the acceptance row counts above are JSON array lengths.

## Disposition

APPROVE. Merge remains gated on explicit operator confirmation per CONTRACT. Do not merge from
this review-verdict step.
