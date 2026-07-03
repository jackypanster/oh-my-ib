# review-01 - stk-orders PR #15

Verdict: APPROVED / merge-ready, NOT merged in this run. Human confirmation still required before
`pipeline-review` may squash-merge PR #15.

Review depth: deep. Reason: first write path in the repo; mutating external brokerage API surface.

Review surface:
- Base: `main` at `d45525258c5f25ba2cbbab7868986941359cbf05`.
- PR head: `ee9291f95535e6c579976b48eb6959c9f6436be7` (`feat/stk-orders`, PR #15 open).
- Feature history reviewed: seq=4 R1 implemented ADR 0017 verbatim; PAPER acceptance hung before
  placement in `next_valid_order_id()`; seq=5 routed ADR 0018; seq=6 R2 replaced allocation with
  `client.next_order_id()`.

Findings:
- Blocking findings: none.
- Non-blocking note: executable allocation is clean (`client.next_order_id()`); textual
  `next_valid_order_id` mentions remain only as frozen/history/comment context. `src/ib/trade.rs`
  line 9 still has an old module-doc phrase naming `next_valid_order_id`; line 152-156 and runtime
  code are correct. Do not treat the historical ADR0018 / frozen-spec mentions as executable
  residue.
- Operational note: order id cancellation is client-id namespaced. Operator observed cross-client-id
  cancel returns structured IB error 10147. Correct behavior; ops must cancel from the same client-id
  namespace that placed the order.

Deterministic gates:
- `git pull --rebase`: already up to date.
- Freeze gate 1: `git diff --exit-code 3692c71bc11e873b2be8f3c9448a2a8d4f4d9e8f ee9291f95535e6c579976b48eb6959c9f6436be7 -- tests/stk_orders_command.rs` -> empty.
- Freeze gate 2: `git diff --exit-code main ee9291f95535e6c579976b48eb6959c9f6436be7 -- tests/` -> empty.
- Scope gate: `git diff --name-only main...ee9291f95535e6c579976b48eb6959c9f6436be7` -> exactly
  `AGENTS.md`, `CLAUDE.md`, `src/cli.rs`, `src/ib/mod.rs`, `src/ib/trade.rs`, `src/main.rs`.
- Diff whitespace gate: `git diff --check main...ee9291f95535e6c579976b48eb6959c9f6436be7` -> clean.

Isolated verification:
- Worktree: `/tmp/codex-review-wt3`, detached at `ee9291f95535e6c579976b48eb6959c9f6436be7`.
- Commands: `cargo build && cargo test && cargo clippy --all-targets -- -D warnings`.
- Result: pass. `cargo test` passed 114 tests total, including `tests/stk_orders_command.rs` 16/16.
- Cleanup: `/tmp/codex-review-wt3` removed after verification.

Semantic review:
- Containment: write API symbols `place_order|submit_order|encode_place_order|cancel_order` in source
  are executable only in `src/ib/trade.rs`; no read command calls trade gateway fns. `src/main.rs`
  dispatches trade only from `Command::Buy`, `Command::Sell`, `Command::Cancel`.
- Ordering invariant: `src/ib/trade.rs` validates quantity/limit before `require_live_write_gate`,
  gates before `super::connect`, allocates id after connect, then builds and places the order.
- Live gate: `require_live_write_gate` gates on `cfg.port == LIVE_PORT` and
  `OMI_ALLOW_LIVE=1`. Existing config layer also rejects live port without `--live` before any command
  dispatch, preserving the double gate.
- Ack wait: placement uses `subscription.timeout_iter_data(TAKE_FIRST_TIMEOUT)` and takes first
  `OrderStatus` or `OpenOrder`; `ExecutionData`/`CommissionReport` are skipped. `None` maps to an
  UNKNOWN timeout envelope naming the allocated order id and saying to verify with `omi orders` and
  not retry blindly. Cancel uses one bounded `CancelOrder::OrderStatus` read and the same no-blind-retry
  posture.
- No retry: no re-placement or retry branch exists in `src/ib/trade.rs`; only comments mention blind
  retry as forbidden.
- R2 allocator: `src/ib/trade.rs` uses `let order_id = client.next_order_id();`. ibapi 3.1.0 source
  confirms `Client::new` seeds `ClientIdManager` from `connection_metadata.next_order_id`; `next_order_id`
  is local atomic fetch-add; `next_valid_order_id()` sends `RequestIds` and blocks in
  `subscription.next()`.
- Pure seams: `build_stk_order` emits STK `Contract::stock`, LMT/MKT, TIF Day, action, quantity, and
  limit fields as frozen. `shape_order_ack` emits exactly six keys with MKT limit as null.
- Docs amendment: `AGENTS.md` and `CLAUDE.md` contain the arch.md Docs amendment text. CLAUDE.md is
  794 bytes and keeps the old public/secret backstop plus the live-write gate information in the merged
  Phase 2 bullet.
- Public repo scan: no new account ids, tokens, balances, or secrets found in the PR diff. Existing
  test fixtures contain fake account-like strings.

PAPER acceptance evidence:
- `/tmp/o1.json`: buy ack `{"order_id":1,"status":"PreSubmitted","symbol":"AAPL","action":"Buy","quantity":1.0,"limit_price":50.0}`; operator reported 4.1s.
- `/tmp/o3.json`: cancel ack `{"order_id":1,"status":"Cancelled"}`; operator reported 1.2s.
- Operator reported full lifecycle evidence: order visible after place, completed-orders shows
  Cancelled after cancel, positions unchanged.

Disposition:
- PR #15 is ready for the human-confirmed merge step.
- Do not merge automatically. Next `pipeline-review` continuation must re-check PR head, then
  squash-merge only after explicit human confirmation; after merge, flip card 01 to done, set
  `current.json.stage=done`, append the done journal entry, commit, and push main.
