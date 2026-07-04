# Run journal — option-close

## seq=1 · 2026-07-04T07:37:49Z · prd→arch · completed · by=claude-fable-5
done:   PRD decision-complete: close-by-conid verb (side/qty derived from held position,
        LMT/DAY, ADR 0017/0018 verbatim) + positions/brief 14-key row identity enrichment
        (sec_type/expiry/strike/right/multiplier, nulls on non-OPT). D4 rebuild+conid-assert
        placement path (portfolio-contract resubmit REJECTED as unverified on Tiger).
        Operator /think 2026-07-04: scope + full-auto authorized.
output: .pipeline/option-close/PRD.md
--- handoff ---
>>> NEXT
Run pipeline-arch on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
Model: frontier SOTA required — operator assigns the bot; the pipeline can't verify the model.
First: git pull --rebase; no repo .env (real config outside repo, not needed for arch).
Read for context (before acting):
  - AGENTS.md — repo conventions (agent-first authoring, hard safety rules)
  - .pipeline/option-close/PRD.md — what + locked decisions D1-D8
  - src/ib/positions.rs — position_row shared seam (brief.rs consumes it — parity ripple)
  - src/ib/trade.rs — place_with_client core, option builder path, validation-ordering idiom
  - src/ib/option_quote.rs — pub(crate) parse_expiry/normalize_right (reuse, ADR 0020 D6)
  - docs/adr/0017..0021 — write-path safety machinery (verbatim reuse, zero new machinery)
Your task (concrete, numbered):
  1. grill-with-docs the design tree: (a) position_row 9→14 key mapping (SecurityType→string,
     null semantics, brief parity), (b) option_close single-connect data flow
     (drain→match→rebuild→contract_details conid assert→place_with_client), (c) exact
     OptionCloseArgs/CLI + 10-key ack struct, (d) AGENTS.md+CLAUDE.md two-text amendment.
  2. Compute the CLAUDE.md <900B budget BEFORE freezing amendment text (option-orders lesson).
  3. Emit arch.md + CONTEXT.md update + ADR 0022; commit once with journal seq=2; push.
Feature gotchas (project-specific traps the next node MUST know):
  - Single-connect invariant: ONE client for drain+resolve+place (option-combo review lesson —
    a second same-client-id connect wedges the gateway).
  - positions row shape is SHARED with brief — two frozen suites re-freeze (task owns).
  - SecurityType/OptionRight are non_exhaustive ibapi enums; multiplier is String; expiry row
    output is raw passthrough string but the rebuild needs parse_expiry's (y,m,d).
  - expiry/strike/right come from ibapi decode_contract on the portfolio stream — Tiger's actual
    field content is UNVERIFIED (PRD risk 1); D4 conid assert is the guard, keep it before place.
Done when: arch.md + CONTEXT.md + ADR 0022 on trunk (journal seq=2 riding the same commit).
On success: run pipeline-task. On failure: attempts++; >=3 ⇒ blocked ⇒ run pipeline-hunt.
<<< END
