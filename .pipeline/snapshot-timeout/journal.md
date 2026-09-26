# Run journal — snapshot-timeout

## seq=1 · 2026-09-26T07:04:14Z · ∅→prd · completed · by=cc/claude-opus-5-5
done:   PRD for snapshot-timeout: bound the two bare SnapshotEnd drains (quote.rs:76, option_quote.rs:215)
        with a fixed 20s total deadline → existing `timeout` error, exit 6, whole-command fail-fast.
        Live-disproven premise (read-timeouts Non-scope "bounded by SnapshotEnd"): live quote hangs on :4001.
        2 human-confirmed (D1 fail-fast, D7 coordinated/herdr) + 4 code-verified + 2 ⚠️ assumed (D2 20s, D4 wording).
        control.json (coordinated, human-direct merge) written on explicit operator request.
output: .pipeline/snapshot-timeout/PRD.md · .pipeline/snapshot-timeout/control.json
--- handoff ---
>>> NEXT
Run pipeline-arch on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
Model: frontier SOTA required (CC role) — operator assigns the bot; the pipeline can't verify the model.
Mode: coordinated (control.json) — herdr panes: CC=prd/arch/task, Pi=impl, Codex=review+merge on human-direct token.
First: git pull --rebase; no .env in this repo (runtime config lives at ~/.config/oh-my-ib/config.toml — never commit it).
Read for context (before acting):
  - AGENTS.md — agent-first output/error contract + hard safety rules (read FIRST)
  - .pipeline/snapshot-timeout/PRD.md — what (decisions are provenance-tagged)
  - src/ib/quote.rs — quote_one bare drain (:76), ADR 0013 comments (:40-45)
  - src/ib/option_quote.rs — bare drain (:215), ADR 0019 D2 comments (:4-5, :212)
  - src/ib/mod.rs:60 TAKE_FIRST_TIMEOUT; src/error.rs:42,65 AppError::timeout
  - src/ib/option_chain.rs / completed_orders.rs — ADR 0016 Instant-classified timeout precedent
  - ibapi-3.1.0 src/subscriptions/sync.rs:78-99 (cancel), :222-240 (next_timeout), :284-289 (Drop)
  - .pipeline/read-timeouts/{PRD.md,docs/adr/0012-take-first-timeout-twin.md} — prior timeout decisions
Your task (concrete, numbered):
  1. grill-with-docs the PRD against the codebase; ⚠️ assumed D2 (20s, >IB ~11s snapshot bound, do NOT
     reuse TAKE_FIRST_TIMEOUT=10s) and D4 (message wording) are MANDATORY challenge targets.
  2. Pin the deadline-loop mechanism over next_timeout(remaining): Notice handling must match iter_data
     (filter + warn), Err ⇒ data error unchanged, SnapshotEnd ⇒ success, None-on-expiry ⇒ timeout.
  3. Emit arch.md + CONTEXT.md + ADR (next free number) amending ADR 0013 / ADR 0019 D2 drain posture.
  4. Name the pure freezable seams (deadline const, timeout-error builder) for pipeline-task.
Done when: arch.md + CONTEXT.md + ADR committed with journal seq=2, current.json.stage=arch.
<<< END
