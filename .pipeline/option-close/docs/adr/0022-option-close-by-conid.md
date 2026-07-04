# ADR 0022 — option-close: close-by-conid, position-derived side

Status: accepted (2026-07-04, feature option-close)
Context: PRD `.pipeline/option-close/PRD.md`; extends ADR 0017/0018/0020/0021 (all VERBATIM).

## Decision

1. **Close is addressed by conid, not by option four-tuple.** The user/agent passes
   `--conid` from `omi positions`; the verb matches it against the LIVE portfolio stream
   (`account_updates` drain, LAST match wins). No held match / position 0 ⇒ `not_found`,
   nothing placed. Rationale: the position match IS the anti-open gate — a re-typed
   four-tuple can open a new position on a typo; a conid that is not held cannot place
   anything at all.
2. **Order side is DERIVED, never supplied**: `position > 0 ⇒ SELL`, `position < 0 ⇒ BUY`
   (pure frozen seam `derive_close`). `--qty` defaults to |position|; over-close ⇒ usage
   error (a close never flips a position through zero). Rationale: side inversion on manual
   close DOUBLES exposure — the highest-blast-radius agent error this repo can prevent.
3. **Rebuild + assert placement path**: the placement contract is rebuilt from the matched
   row's decoded identity via `build_option_order` (live-proven builder chain, ADR 0020 D8),
   then `contract_details` FIRST-row conid must equal `--conid` (ADR 0021 resolve pattern)
   or `data`-fail BEFORE placing. REJECTED alternative: resubmitting the portfolio-decoded
   `Contract` verbatim, or conid-only placement (protocol supports it — PLACE_ORDER_CONID —
   but it is UNVERIFIED on the Tiger gateway; the assert converts any identity drift into a
   clean pre-order failure instead of a wrong order).
4. **Row identity enrichment is flat-with-nulls on the shared seam**: `position_row` gains
   `sec_type` (always, `SecurityType` Display wire code) + `expiry`/`strike`/`right`/
   `multiplier` (populated iff OPT, else `null`; empty multiplier ⇒ null; non_exhaustive
   right fallback ⇒ null). Additive keys, existing 9 untouched ⇒ backward-compatible for
   JSON consumers; `brief` parity is automatic (same fn). `position_row` promoted
   `pub(crate)` → `pub` as the frozen test seam (first-ever freeze of this shape).
5. **LMT/DAY only, single-connect, safety parity**: `--limit` required finite > 0; ONE
   client for drain + assert + place; gate/allocator/bounded-first-ack/no-retry VERBATIM.

## Consequences

- Closing a short option (buy-to-close) is exactly as safe as closing a long — the sign
  does the work; agents stop encoding side logic in prompts.
- A stale conid (position closed elsewhere between `positions` and `option-close`) fails
  closed as `not_found` — never re-opens.
- Combo/BAG structures close leg-by-leg through this verb (whole-structure close remains
  out of scope, PRD non-scope).
- The `account_updates` drain adds one bounded read to every close (~the cost of
  `omi positions`) — accepted: correctness over latency on a write path.
