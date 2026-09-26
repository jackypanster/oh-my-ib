# PRD — snapshot-timeout

Feature: bound every market-data SNAPSHOT drain with a total deadline so `omi quote` (single +
batch) and `omi option-quote` can never block forever when the gateway never sends `SnapshotEnd`.
Status: decision-complete (grilled 2026-09-26; operator locked D1 + operating mode; mechanism
verified against this repo's source and ibapi-3.1.0 crate source, not guessed). Read-only feature —
`src/ib/trade.rs` untouched; write gates unaffected.

## Problem

The two snapshot drains iterate `subscription.iter_data()` bare until `TickTypes::SnapshotEnd`,
deliberately NOT timeout-wrapped:

- `src/ib/quote.rs:76` (`quote_one`, shared by single and variadic `omi quote`; ADR 0013 comment
  `quote.rs:40-45`).
- `src/ib/option_quote.rs:215` (`option_quote`; ADR 0019 D2 comment `option_quote.rs:4-5, 212`).
- `.pipeline/read-timeouts/PRD.md` Non-scope excluded `quote` as "bounded by SnapshotEnd".

That premise is now **live-disproven** (2026-09-26, Saturday/market closed, IBKR IB Gateway 10.45 on
`:4001`, freshly restarted, after signing the market-data API acknowledgement):

- `omi --live --md-type live quote NVDA` → no error, no ticks, no `SnapshotEnd`; killed at 120s.
- `omi --live --md-type live quote SPY` → same; killed at 40s. Reproduced twice (before and after
  gateway restart). Delayed-mode `quote` completes normally (~12.7s wall incl. connect).
- Consequence: the agent-facing CLI hangs indefinitely; the consuming LLM agent gets no signal.

These are the ONLY two snapshot call sites (`grep "market_data(\|\.snapshot()" src` → quote.rs:70,
option_quote.rs:207). All other streaming reads are already bounded (ADR 0012 take-first twin,
ADR 0016 End-bounded drains).

## Goal

Every snapshot drain completes in bounded time. On deadline expiry the command exits promptly with
the existing structured `timeout` error (exit 6) naming the offending instrument and the likely
causes, so the agent can machine-distinguish "no snapshot data arrived" from data/connection errors.

## Success criteria (acceptance)

1. **Bounded single quote**: `omi --live --md-type live quote NVDA` in the reproducing state
   (market closed / no RT snapshot) exits within ~deadline + connect overhead (≤ ~25s) with
   `{"error":{"code":"timeout","context":"quote/NVDA",…}}` on stderr, nothing on stdout, exit 6.
2. **Bounded batch**: `omi --live --md-type live quote AAPL NVDA` — the first symbol whose snapshot
   exceeds the deadline fails the WHOLE command (no partial stdout), error context
   `quote/<that symbol>`, exit 6.
3. **Bounded option-quote**: an `option-quote` whose snapshot never ends exits with code `timeout`,
   context `option-quote`, exit 6, message naming the contract (symbol/expiry/strike/right).
4. **No regression on the healthy path**: `omi --live --md-type delayed quote SPY` (and N=1
   byte-identical output shape, ADR 0013 red line) still succeeds with ticks; existing frozen tests
   stay green.
5. **No orphaned market-data line**: the timed-out subscription is cancelled (dropped before the
   next symbol / before exit) — see D5.
6. **Verify gate**: `cargo build` · `cargo clippy --all-targets -- -D warnings` · `cargo test` green.

Criteria 1–3 and 5 are gateway-dependent ⇒ review-by-reading + operator live acceptance on `:4001`
(the weekend state reproduces the hang deterministically, so acceptance does not need RTH).

## Scope

- Replace the bare `iter_data()` in both snapshot drains with a TOTAL-deadline drain (one deadline
  per snapshot request, measured from subscribe) — mechanism per D3.
- One shared fixed deadline constant for both drains (D2).
- Timeout ⇒ `AppError::timeout` (existing, `src/error.rs:42`) with actionable message (D4).
- Update the stale "deliberately NOT timeout-wrapped" comments/doc references in quote.rs and
  option_quote.rs to point at the new ADR (arch assigns the number).

## Non-scope

- Partial-tick output on timeout (rejected, D1).
- Configurability of the deadline (no flag, no config key) — D2.
- Any change to non-snapshot reads, `trade.rs`, write gates, or output JSON shapes on success.
- Market-data entitlement handling (OPRA/subscription errors 354/10091/10168 already surface as
  `data` errors fast; unchanged).
- Batch option quotes / `option-board` (separate future PRD, gated on OPRA subscription).

## Deferred (future PRDs)

- `option-board` — batch streaming option quotes + greeks + OI for a watchlist: waiting on OPRA L1
  subscription (operator's non-pro status review pending since 2026-09-26); no live data to accept against.
- Default `md_type` for `--live` sessions (live vs delayed): orthogonal UX decision, not needed to
  fix the hang.

## Decisions

- D1 **Timeout ⇒ whole-command fail-fast error, not partial ticks** — `✅ human-confirmed`
  (operator, 2026-09-26: "默认"). Consistent with ADR 0013 operator D3 (batch: first error fails
  all, no partial output; `quote.rs:41-42`) and read-timeouts D2 (`timeout` code, exit 6). Success
  output shape: zero change.
- D2 **Fixed 20s total deadline per snapshot, one shared const, not configurable** — `⚠️ assumed`
  (low-risk default, announced to operator, no objection). Rationale: IB closes a regular snapshot
  after ~11s if not all ticks arrive (IB API docs — external, not repo-verified); observed healthy
  delayed quote ≈12.7s wall incl. connect+md-type switch (2026-09-26). 20s leaves margin. Pattern
  mirrors read-timeouts D3 (fixed const, no knob). **Arch challenge target**: confirm the ~11s IB
  snapshot bound and whether `TAKE_FIRST_TIMEOUT` (10s, `src/ib/mod.rs:60`) must NOT be reused here
  (10s < 11s ⇒ would false-timeout healthy snapshots).
- D3 **Mechanism = total deadline over `Subscription::next_timeout(remaining)`** — `📖 code-verified`
  (ibapi-3.1.0 `src/subscriptions/sync.rs:222-240`: `next_timeout` returns `None` on expiry;
  notices must be filtered — `next_timeout` yields `SubscriptionItem` incl. `Notice`, unlike
  `iter_data`). NOT `timeout_iter_data(d)` — that is a PER-ITEM window and would let a trickle of
  ticks extend the drain unboundedly and mis-bound the healthy ~11s tail. Arch pins the exact loop +
  Notice handling (Notice semantics must match current `iter_data`, which filters+warn-logs them).
- D4 **Error envelope** — `📖 code-verified` shape (`AppError::timeout`, `src/error.rs:42,65`; exit 6
  per read-timeouts D2). Context: `quote/<symbol>` (ADR 0013 convention) / `option-quote`. Message
  (`⚠️ assumed` wording, low-risk): names the instrument, the deadline, the md-type, and the likely
  causes — "no real-time snapshot entitlement for this instrument, or market closed with no ticks;
  try --md-type delayed". Arch may refine wording; must stay single-line, agent-parseable.
- D5 **Timed-out subscription is cancelled by drop** — `📖 code-verified` (ibapi-3.1.0
  `sync.rs:78-99` `cancel()` skips ONLY when `snapshot_ended`; `sync.rs:284-289` `Drop` calls
  `cancel()` ⇒ a timed-out snapshot sends CancelMktData on drop). Requirement: the subscription must
  be dropped before the next batch symbol subscribes / before exit (no extra line held).
- D6 **Scope = the two snapshot drains only** — `📖 code-verified` (grep above; brief.rs/signal.rs/
  sma_tick.rs have no `market_data(` call).
- D7 **Operating mode = coordinated (herdr)** — `✅ human-confirmed` (operator, 2026-09-26: "应该是通过
  herdr去编排 … 我可以帮你提前启动pi和codex"). `control.json` written in this commit;
  `merge_gate: human-direct` — only Codex merges, only on the operator's direct token in its own pane.

## Freeze coverage (preflight for pipeline-task)

Plausible hermetic seams (no gateway, no new mocks/stubs — repo rule):
- The deadline const value (`== 20s`, and `> 11s`) — pure assertion.
- A pure timeout-error builder (instrument + md-type + deadline → `AppError` with code `timeout`,
  context, message fields) — offline-testable like `shape_option_quote`.
- Existing dead-port black-box tests still route `quote`/`option-quote` to `connection` (unchanged
  pre-connect validation ordering).
- The deadline-loop against a live gateway (criteria 1–3, 5) is NOT hermetically freezable ⇒
  review-by-reading + operator live acceptance (weekend-reproducible), same posture as ADR 0016/0019.

## Risks

- R1: if IB's snapshot tail is longer than assumed on some instruments (e.g. option greeks) a 20s
  deadline could false-timeout — mitigated by D2 arch challenge + live acceptance on a healthy
  delayed quote.
- R2: `next_timeout` Notice semantics differ from `iter_data` (Notices are filtered there) —
  mis-handling could turn benign notices into errors. Arch must pin.
