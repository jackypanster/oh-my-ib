# review-01 - completed-orders (PR #14, card 01)

Verdict: **APPROVE** - awaiting explicit operator merge confirmation. No blocking findings.
This review covers both implementation rounds on PR #14: the original ADR 0015 additive
subcommand and the ADR 0016 bounded-drain fix after live acceptance found the
`CompletedOrdersEnd` wedge.

Reviewed head: `46a12b054cf99cdf244c58d8ff1555a961acef2e` (`feat/completed-orders`, PR #14).
Spec-rev: `aff35991c759a0bbfd44dd03ea1d67fac0241dbf`. Card 01 status at review time:
`review`, attempts `1`.

## Deterministic gates

- **Freeze gate: EMPTY.** `git diff aff35991c759a0bbfd44dd03ea1d67fac0241dbf 46a12b054cf99cdf244c58d8ff1555a961acef2e -- tests/completed_orders_command.rs` produced no output.
- **Whole tests tree untouched:** `git diff main 46a12b0 -- tests/` produced no output.
- **Scope exact:** `git diff --stat main...46a12b0` is exactly the card's four impl-paths:
  `src/cli.rs`, `src/ib/completed_orders.rs`, `src/ib/mod.rs`, `src/main.rs`
  (4 files, +135).
- **Whitespace check:** `git diff --check main...46a12b0` produced no output.
- **PR head confirmed:** `gh pr view 14 --json headRefOid,baseRefName,headRefName,state,url,title`
  returned head `46a12b054cf99cdf244c58d8ff1555a961acef2e`, base `main`, head branch
  `feat/completed-orders`, state `OPEN`.
- **Full-suite gate: GREEN** on detached worktree `/tmp/codex-review-wt2` at `46a12b0`:
  `cargo build` passed; `cargo test` passed (98 passed, 0 failed);
  `cargo clippy --all-targets -- -D warnings` passed.

## Semantic review

Direct review surface: `git diff main...46a12b0`, direct `git show 46a12b0:<path>` reads,
`src/ib/orders.rs`, arch.md section "Component design" + "Amendment", ADR 0015, and ADR 0016.

- `src/cli.rs`: adds only `Command::CompletedOrders`; no per-command flags added.
- `src/ib/mod.rs`: adds only `mod completed_orders;` and re-exports
  `completed_orders`, `shape_completed_orders`, `CompletedOrderRow`.
- `src/main.rs`: adds only `Command::CompletedOrders => ib::completed_orders(&config)`.
- `shape_completed_orders` emits the frozen 14-key row set in gateway order, with
  `limit_price`/`aux_price` raw `Option<f64>` values (`None` -> `null`) and empty strings
  passed through.
- Gateway call is `client.completed_orders(false)`, hardcoded as required by ADR 0015.
- Filter semantics match `orders.rs`: rows are filtered only when `cfg.account` is explicitly
  set; unset means pass-through, with no auto-resolution.
- Field mapping matches the pinned row contract: `order_id`, `account`, `symbol` via
  `.to_string()`, `conid`, `action` Debug, `quantity`, `order_type`, `limit_price`,
  `aux_price`, `tif` Debug, `status` Debug, `filled_quantity`, `completed_time`,
  `completed_status`.
- Error contexts are `completed-orders`; request failure maps to `data`, stream failure maps
  to `data`, and ADR 0016 timeout maps to `timeout` (exit 6 via existing `AppError`).
- ADR 0016 drain fix is implemented as specified: `timeout_iter_data(super::TAKE_FIRST_TIMEOUT)`,
  `Instant::now()` immediately before each `.next()`, four arms:
  `Some(Ok(OrderData))` filter+push, `Some(Ok(_))` skip, `Some(Err)` data envelope,
  `None` classified by elapsed time (`>= TAKE_FIRST_TIMEOUT` => timeout, otherwise success break).
  `TAKE_FIRST_TIMEOUT` is reused from `src/ib/mod.rs`, not redefined.
- `output.rs`, `error.rs`, `brief.rs`, and `orders.rs` are untouched.
- ibapi source read confirms `completed_orders(false)` returns `Subscription<Orders>`,
  `CompletedOrdersEnd` decodes to EndOfStream, and `timeout_iter_data` reaches `next_timeout`;
  timeout and EndOfStream both surface as `None`, which is why ADR 0016's elapsed-time
  classification is needed.

## Read-only / safety review

- Precise API grep over the PR diff for `.place_order`, `.what_if_order`, `.modify_order`,
  `.cancel_order`, `reqGlobalCancel`, and related variants produced no matches.
- Broader grep hits only prose or row/state text such as `filled/cancelled`, not trading API calls.
- Secret scan of the PR diff for account ids, tokens, secrets, balances produced no matches.
- The command is read-only request/drain/emit; no filesystem mutation, order placement,
  modification, cancellation, or global cancel surface is introduced.

## Live acceptance evidence

Historical context from journal:

- Seq=5 records two pre-fix live hangs on a healthy gateway (>2.5m and >45s), with health OK
  between attempts. This was classified as a drain posture gap, not a row-mapping bug, and
  produced ADR 0016 plus the Round 2 bounded drain.
- Seq=6 records the Round 2 fix: bounded drain only, frozen spec untouched, 98/98 + clippy
  clean, read-only grep clean.

I read the operator-provided post-fix JSON files:

- `/tmp/co3.json`: `{"completed_orders":[]}`; `jq` confirmed the wrapper exists, the value is an array,
  and length is 0. Operator reported exit 0 in 2.8s.
- `/tmp/co4.json`: `{"completed_orders":[]}`; `jq` confirmed the same wrapper/array/empty shape.
  Operator reported exit 0 in 188ms.

Conclusion: PRD criterion 8 amended path (a) is directly satisfied: the gateway answered and
the command exited 0 with the wrapper shape. The earlier wedge is intermittent; ADR 0016's
bounded drain remains the standing defense for amended path (b).

## Non-blocking advisory

- `src/ib/completed_orders.rs:5-9` still has a module-level ADR 0015-era sentence saying the
  class uses `iter_data()` and "no TAKE_FIRST_TIMEOUT". The function-level comment and code
  below it correctly document and implement ADR 0016. This is not a behavior blocker, but the
  module header should be corrected on the next touch so future agents do not read the stale
  summary in isolation. The frozen test's prose has the same historical limitation and should
  not be edited by review.

## Disposition

APPROVE. Merge remains gated on explicit operator confirmation per CONTRACT. Do not merge from
this review-verdict step.
