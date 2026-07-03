# review-01 — brief-command card 01 (PR #10, head 6808d09)

Verdict: **APPROVE** — awaiting explicit operator merge confirmation. Live acceptance of
`omi --live brief` PASSED on a healthy gateway; a live-acceptance side-finding (pre-existing
reqPnL wedge class, ADR 0007's recorded hang scenario) is documented below with a routing
recommendation — it is NOT caused by this diff.

## Deterministic gates

- **Freeze gate: EMPTY.** `git diff f7cab5d..6808d09 -- tests/brief_command.rs` = 0 lines;
  `git diff origin/main..6808d09 -- tests/` = 0 lines (the impl-stage cargo-fmt incident was
  reverted before commit, as journal seq=4 records; verified independently here).
- **Full-suite gate: GREEN** on a detached worktree at 6808d09: `cargo build` clean,
  `cargo test` 64 passed / 0 failed (incl. the formerly-red frozen spec 7/7),
  `cargo clippy --all-targets -- -D warnings` clean.
- PR surface: 10 files, ~417 lines, src-only. Cargo.toml/Cargo.lock untouched (no new deps).

## Semantic review (session + independent adversarial subagent)

Adversarial pass attempted to refute 7 claims; ALL CONFIRMED with citations:

1. Sibling behavior byte-identical (account/positions/pnl/pnl_by_position/orders/executions):
   key sets, error strings, orders `--account` filter semantics, currency first-seen rule all
   preserved; serde_json has no `preserve_order` (BTreeMap) so insert-after-build cannot reorder
   serialized keys.
2. ADR 0010 order + no concurrent subscriptions verified in brief.rs:56-103; ibapi routing domains
   disjoint (CommissionsReport = ByExecutionId only, routing.rs:132 + transport/sync.rs:447-457 —
   no shared fallback; unsolicited errors go to the notice broadcaster, cannot poison the next
   subscription).
3. Take-first vs drain-to-End per stream matches each decoder's message set (PnL/PnLSingle have
   NO End marker; AccountUpdate::End / OpenOrderEnd / ExecutionDataEnd exist).
4. `assemble_brief` = exact 8-key pass-through, frozen test byte-identical on the branch.
5. as_of: `server_time()` is UTC by construction (`from_unix_timestamp`); zero-padded ISO-8601;
   `u8::from(Month)` infallible; no panic path.
6. Fail-fast: all sections `?`-propagate before any stdout; contexts brief/as_of,
   brief/account_summary, brief/pnl, brief/pnl_by_position, brief/orders, brief/executions.
7. Hygiene: no account-id/token patterns in the diff; tests/ untouched.

ADVISORY (info): with `--account` unset, the `orders` section is unfiltered while other sections
are scoped to the resolved account — explicitly ADR 0011's decision (verbatim `omi orders`
semantics); revisit only if brief must become fully account-scoped in a multi-account future.
External signal: CodeRabbit pass, no inline findings.

## Live acceptance (PRD criterion 10) — operator gateway, live :4001, 2026-07-03

- `omi --live health` ✓ connected, server v221.
- **`omi --live brief` ✓ PASS** (fresh gateway session): one connection, 3.6 s, exit 0; all 8
  top-level keys; `as_of` valid ISO-8601 matching server time; numeric account_summary + pnl
  (no sentinel leak); flat account ⇒ `[]` for pnl_by_position/positions/orders/executions
  (criterion 6 shape). Account ids/balances redacted — public repo.
- Cross-check vs individual commands, same gateway session: `account` matched brief's
  account_summary EXACTLY (all five fields); `positions`/`orders`/`executions`/`pnl-by-position`
  each returned the same `[]` payloads as brief's sections. 5/6 commands cross-checked clean.
- **Side-finding (NOT this diff): standalone `omi --live pnl` WEDGED** — blocking `next_data()`
  never returned (reproduced twice, incl. with a fresh `--client-id`, before any process kill;
  first occurrence was immediately after `omi account`'s connect/disconnect cycle). After the
  wedged processes were killed (cancel never sent ⇒ orphaned reqPnL subscriptions), a SECOND
  `omi --live brief` also wedged at its pnl step — the gateway's PnL channel stays polluted until
  a gateway restart. Attribution: the blocking-read-without-timeout behavior is PRE-EXISTING
  (pnl-command feature, ADR 0007 — which explicitly recorded `next_timeout(Duration)` as the
  fallback "applied only if live acceptance shows hangs"). That trigger condition HAS NOW FIRED
  in live acceptance. This PR's refactor is byte-identical on that path (adversarially confirmed)
  and brief passed acceptance on a healthy gateway; the wedge is a gateway-state hazard all
  reqPnL consumers share (this gateway build: 2026-06-25 stable, new install on the operator's
  M1 machine; yesterday's mac-mini session did not exhibit it).

## Disposition

- APPROVE stands on: all gates green + adversarial semantic pass + brief's own live acceptance
  PASS + 5/6 cross-check + the wedge being a pre-existing, diff-independent class.
- **Recommended routing for the wedge (operator decides):** merge this PR on confirm, then start
  a NEW pipeline feature (`read-timeouts`) via pipeline-prd to apply ADR 0007's recorded
  `next_timeout` fallback to ALL take-first reads (pnl, pnl_single — both standalone and inside
  brief), now that the hang trigger is live-proven. Alternative (rejected as scope-mixing): an
  append-card on brief-command would harden only brief while leaving `omi pnl` /
  `omi pnl-by-position` exposed to the same wedge.
- Merge remains gated on: explicit operator confirmation (+ optionally one clean re-acceptance of
  `omi --live brief` after a gateway restart, recommended since the current gateway session is
  polluted by the diagnostic kills).
