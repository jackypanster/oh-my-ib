# review-01 — multi-quote (PR #12, card 01)

Verdict: **APPROVE** — awaiting explicit operator merge confirmation. All deterministic gates
green; semantic + adversarial review clean (one ADR-doc errata recorded below, behavior
unaffected); live acceptance PASSED for single AND batch in the same gateway session
(PRD criterion 9).

Reviewed head: `5437831` (feat/multi-quote). Spec-rev: `828348a`. Impl by: interactive π/omp
(operator-assigned per CONTRACT Model line; watched live by the operator).

## Deterministic gates

- **Freeze gate: EMPTY.** `git diff 828348a..5437831 -- tests/multi_quote.rs` = 0 lines; whole
  `tests/` vs main = 0 lines.
- **Scope exact:** three-dot diff = the card's 3 impl-paths only (+56 −9);
  `main.rs`/`output.rs`/`error.rs` byte-identical to main.
- **Full-suite gate: GREEN** on a detached worktree at `5437831`: `cargo build` clean,
  `cargo test` **79 passed / 0 failed** (frozen `multi_quote` 8/8 red→green; all pre-existing
  suites incl. `quote_ticks` and `data_commands` untouched green),
  `cargo clippy --all-targets -- -D warnings` clean.

## Semantic review (line-by-line vs arch.md + independent adversarial pass)

Direct read: diff matches arch.md §Component design verbatim — variadic `symbols` with
`required = true`, help contains the frozen `symbol(s)` literal, `quote_one` is token-for-token
today's assembly with `quote/<symbol>` contexts, `shape_quotes` exactly as pinned, one connect +
one switch, fail-fast `?` loop in input order, NO timeout wrapping on the bounded drain.

Adversarial subagent attempted to REFUTE 6 claims; 5 CONFIRMED, 1 PARTIALLY refuted (doc-only):

1. **N=1 byte-identity: CONFIRMED empirically** — built main and branch binaries and
   byte-compared N=1 outputs: connection envelope, sec-type envelope, bad-flag usage envelope
   all IDENTICAL including exit codes. `quote_one` body is token-for-token main's assembly;
   `shape_quotes` N=1 pops the row itself (no rewrap). Only `--help` text differs, exactly as
   the frozen spec mandates.
2. Clap contract: zero symbols → `code="usage"` exit 64 (MissingRequiredArgument through the
   existing `_` arm, no panic); flags after symbols parse; `--sec-type` values cannot be
   swallowed into the symbol list.
3. One connect (quote.rs:27) + one switch (quote.rs:38), none inside the loop.
4. Sequential line discipline: CONFIRMED (subscription local to `quote_one`, drains to
   SnapshotEnd or `?`-errors, drops before the next request; fresh request-id per call).
   **Errata (doc-only, REFUTED sub-claim):** ADR 0013's rationale says "drop sends
   `CancelMarketData`" — in ibapi-3.1.0 `cancel()` deliberately EARLY-RETURNS after
   `SnapshotEnd` (subscriptions/sync.rs:78-82; the snapshot is server-complete, nothing is
   open), so no cancel message is sent on the happy path — it is sent only on early-error
   exits. The ADR's CONCLUSION (at most one line open, no pacing exposure) holds — more
   strongly than stated. Recorded here per write-set discipline (review does not edit arch
   docs); fold into ADR 0013 at the next arch-stage touch of this feature.
5. Frozen spec honest: identical on both refs; assertions match impl exactly; could not
   compile on main (`shape_quotes` absent, E0432 verified).
6. No scope creep: `TAKE_FIRST_TIMEOUT` absent from quote.rs (plain `iter_data()`); existing
   frozen quote surfaces re-verified green on the branch.

## Live acceptance (PRD criterion 9) — operator gateway, live :4001, same session, 2026-07-03

PR-head binary from the detached worktree. Quote output carries no account data.

- **`omi --live quote AAPL` ✓ PASS** — exit 0, bare object `{delayed, symbol, ticks{…}}`.
- **`omi --live quote AAPL MSFT NVDA` ✓ PASS** — exit 0, bare array, INPUT order
  `[AAPL, MSFT, NVDA]`, every row exactly the single-symbol shape.
- **Cross-check ✓** — the batch's AAPL row ticks identical to the single run's (same session,
  stable delayed data): single-vs-batch consistency proven.
- Latency note (pre-existing, NOT this diff): delayed-data snapshots take ~12–15s each on the
  gateway side (single 16.5s, batch-of-3 37.4s — sequential SnapshotEnd waits). The batch still
  saves N−1 connects; agents should size N knowing ~12s/symbol wall-clock on delayed md-type.

## Disposition

- APPROVE stands on: freeze gate EMPTY + full suite 79/79 + clippy clean + verbatim-arch
  conformance + empirical N=1 byte-identity + 5/6 adversarial confirmations (the 6th refuting
  only an ADR mechanism sentence, behavior intact) + live single+batch same-session PASS.
- Merge gated on explicit operator confirmation (CONTRACT: only pipeline-review merges,
  human-confirmed).
- Follow-ups (operator decides, NOT this PR): (a) fold the claim-4 errata into ADR 0013 on the
  next arch touch; (b) standing reqPnLSingle first-trading-day observation continues; (c) the
  delayed-snapshot latency is gateway-inherent — if the daily flow ever needs faster batches,
  that is a new feature (e.g. md-type live entitlement or concurrent lines), evidence first.
