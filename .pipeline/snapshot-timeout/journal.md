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

## seq=2 · 2026-09-26T07:08:19Z · prd→arch · completed · by=cc/claude-opus-5-5
done:   arch for snapshot-timeout: total-deadline loop over next_timeout(remaining) at both snapshot drains,
        SNAPSHOT_DEADLINE=20s (📖 IB tickSnapshotEnd ≈11s; ≠ TAKE_FIRST_TIMEOUT 10s), pure snapshot_timeout_error
        builder, Instant-classified None, CancelMktData-on-drop. PRD ⚠️ D2/D4 settled. ADR 0038 amends 0013/0019 D2.
output: .pipeline/snapshot-timeout/arch.md · CONTEXT.md · docs/adr/0038-snapshot-total-deadline.md
--- handoff ---
>>> NEXT
Run pipeline-task on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
Model: frontier SOTA required (CC role). Mode: coordinated (control.json) — Pi=impl, Codex=review+merge on human-direct token.
First: git pull --rebase; no .env in this repo (runtime config at ~/.config/oh-my-ib/config.toml — never commit it).
Read for context (before acting):
  - AGENTS.md (read FIRST) · .pipeline/snapshot-timeout/{PRD.md,arch.md,CONTEXT.md,docs/adr/0038-snapshot-total-deadline.md}
  - tests/read_timeouts.rs, tests/multi_quote.rs — frozen-spec style precedent (offline, no mocks)
Your task (concrete, numbered):
  1. ONE card `01-snapshot-deadline`: spec-paths tests/snapshot_timeout.rs; impl-paths src/ib/mod.rs,
     src/ib/quote.rs, src/ib/option_quote.rs, src/error.rs.
  2. Write the red spec per arch.md §Freeze coverage items 1–4 (const, error builder, option label, dead-port/help regressions).
  3. Stub-compile + clippy the spec in a scratch copy before freezing (frozen specs must be clippy-clean).
Done when: card + frozen red spec committed, journal seq=3, current.json.stage=task.
<<< END

## seq=3 · 2026-09-26T07:10:55Z · arch→task · completed · by=cc/claude-opus-5-5
done:   1 card (01-snapshot-deadline). Frozen red spec tests/snapshot_timeout.rs (7 tests; red = unresolved imports
        SNAPSHOT_DEADLINE / snapshot_timeout_error). Verbatim stub-compile: 7/7 pass + clippy -D warnings clean on scratch
        (scratch deleted). spec-rev 75ae0cf. check-cards ok.
output: tests/snapshot_timeout.rs (spec-rev 75ae0cf) · .pipeline/snapshot-timeout/tasks/01.md
--- handoff ---
>>> NEXT
Run pipeline-impl on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none card=01
Model: impl slot (Pi) — operator assigns; must differ from review model. Mode: coordinated (control.json).
First: git pull --rebase; no .env (runtime config at ~/.config/oh-my-ib/config.toml — never commit it).
Read for context (before acting):
  - AGENTS.md (read FIRST) · .pipeline/snapshot-timeout/tasks/01.md (the card — steps + gotchas)
  - .pipeline/snapshot-timeout/arch.md §Shape + §Loop (normative) · docs/adr/0038-snapshot-total-deadline.md
Your task (concrete, numbered):
  1. Branch feat/snapshot-timeout from main; set card 01 in-progress per CONTRACT.
  2. Implement card 01 steps 1–4 in impl-paths ONLY; NEVER edit tests/snapshot_timeout.rs (spec-rev 75ae0cf).
  3. verify: cargo build && cargo test --test snapshot_timeout; then full-verify (cargo test; clippy -D warnings).
  4. Open a PR to main; card → review; journal seq bump.
Feature gotchas (review scrutinizes by reading — NOT frozen): total-deadline loop at BOTH sites over
  next_timeout(remaining) (not timeout_iter_data, not TAKE_FIRST_TIMEOUT); Notice ⇒ skip; None classified by
  Instant::now() >= deadline (timeout) else break-with-ticks; Some(Err) + success output unchanged; subscription
  dropped before next batch symbol; md-type label + instrument label wiring; stale ADR 0013/0019 comments updated.
Done when: PR open, card 01 status=review, verify + full-verify green.
<<< END

## seq=4 · 2026-09-26T08:17:31Z · impl→review · completed · by=pi/deepseek-v4.1-flash
done:   Card 01 green on feat/snapshot-timeout @ a32f1a5, PR #34 open. SNAPSHOT_DEADLINE (20s, total per
        snapshot) + pure snapshot_timeout_error builder in src/ib/mod.rs; both drains (quote_one,
        option_quote) swapped to the total-deadline loop over next_timeout(remaining): Notice skip,
        Instant-classified None, drop ⇒ CancelMktData, md-type + instrument labels wired; stale ADR
        0013/0019 D2 comments updated to ADR 0038; ErrorKind::Timeout doc broadened. Spec untouched
        (75ae0cf). verify 7/7 (cargo build + cargo test --test snapshot_timeout) + whole-suite cargo
        test + clippy -D warnings GREEN. No SKILL-PROPOSAL.
output: src/ib/{mod,quote,option_quote}.rs + src/error.rs on feat/snapshot-timeout @ a32f1a5;
        https://github.com/jackypanster/oh-my-ib/pull/34; tasks/01.md (review + Assumptions);
        .pipeline/current.json (stage=impl)
--- handoff ---
>>> NEXT
Run pipeline-review on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=https://github.com/jackypanster/oh-my-ib/pull/34
Model: frontier SOTA required (review role) — operator assigns the bot; the pipeline can't verify the model.
Mode: coordinated (control.json) — CC=prd/arch/task, Pi=impl, Codex=review+merge on human-direct token.
First: git pull --rebase; no .env in this repo (runtime config lives at ~/.config/oh-my-ib/config.toml — never commit it).
Read for context (before acting):
  - AGENTS.md (read FIRST) — repo conventions + hard safety rules
  - .pipeline/snapshot-timeout/tasks/01.md — card 01: steps, Freeze coverage, Assumptions
  - docs/adr/0038-snapshot-total-deadline.md (§Decision 2 loop is normative) + .pipeline/snapshot-timeout/arch.md
  - tests/snapshot_timeout.rs — the frozen spec (spec-rev 75ae0cf)
  - src/ib/mod.rs, src/ib/quote.rs, src/ib/option_quote.rs, src/error.rs — the PR diff
Your task (concrete, numbered):
  1. Freeze gate (deterministic, FIRST): git diff 75ae0cfd4839bffb715226e89f3dddfa0a2eb040 <PR-head> --
     tests/snapshot_timeout.rs — non-empty ⇒ REJECT (attempts++, card→todo, route impl). Confirm no
     other tests/* changed vs main.
  2. Scope: the PR diff must be exactly impl-paths (src/ib/mod.rs, src/ib/quote.rs,
     src/ib/option_quote.rs, src/error.rs).
  3. Full-suite gate: on the PR head run current.json.full-verify exactly — cargo build && cargo test
     (whole suite, unfiltered) + cargo clippy --all-targets -- -D warnings.
  4. Semantic read (card Freeze coverage names the read-targets; no hermetic seam — a silent snapshot
     needs a fake IB server): total-deadline loop over next_timeout(remaining) at BOTH sites (NOT
     timeout_iter_data, NOT TAKE_FIRST_TIMEOUT); Notice ⇒ skip (iter_data parity); None classified by
     Instant >= deadline (timeout) vs before-deadline break-with-ticks; Some(Err) arm + success output
     unchanged (N=1 byte-identity, ADR 0013); timed-out subscription dropped before the next batch
     symbol (CancelMktData); md-type label live|delayed|frozen + instrument label wiring; stale ADR
     0013/0019 D2 comments now cite ADR 0038.
  5. Verdict → reviews/review-01.md. APPROVE ⇒ run EVERY pre-merge guard first (freeze gate, semantic
     review, every-card-review completeness, full-suite GREEN on feat/snapshot-timeout HEAD), then
     publish the ONE atomic commit per CONTRACT §Atomic review outcome. Do NOT merge without explicit
     human confirmation. REJECT ⇒ attempts++, card→todo, journal status=failed, route pipeline-impl
     with findings.
Feature gotchas:
  - Card 01 is the feature's ONLY card — completeness guard trivially passes; review runs ONCE on the
    whole feature (PR #34).
  - Operator live acceptance (needs the :4001 gateway; not a hermetic gate): omi --live --md-type live
    quote NVDA ⇒ exit 6 within ~25s, stderr code timeout, context quote/NVDA; omi --live --md-type
    delayed quote SPY ⇒ success with ticks. Record the evidence before merge.
  - Public repo: never commit tokens/account ids; runtime config is ~/.config/oh-my-ib/config.toml.
Done when: review-01.md written; if human-confirmed + live-accepted: PR #34 squash-merged, card 01 done,
stage=done, next journal seq pushed. On reject: card→todo/attempts=1, route pipeline-impl.
<<< END
