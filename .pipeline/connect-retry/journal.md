# journal — connect-retry (review-05 follow-up B)

## seq=1 · 2026-06-28T15:36:38Z · prd→arch · completed · by=claude-opus-4-8(claude-code)
done:   New feature for follow-up B: ib::connect should retry transient EAGAIN/WouldBlock connection
        failures (seen back-to-back account/positions, same client_id) with short backoff; fail fast on
        permanent errors (refused). Classify by std::io::ErrorKind via ibapi::Error::Io (confirmed
        variant), not string matching. Decision-complete PRD written; current.json repointed.
output: .pipeline/connect-retry/PRD.md, .pipeline/current.json
--- handoff ---
>>> NEXT
Run pipeline-arch then task then impl (rebuild from repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
Read: .pipeline/connect-retry/PRD.md; src/ib/mod.rs (connect); review-05.md (finding B).
Design: pub fn is_transient_io(kind: std::io::ErrorKind)->bool (WouldBlock|Interrupted|TimedOut);
  connect retry loop (3x, 250ms*attempt) on Err(ibapi::Error::Io(e)) with is_transient_io(e.kind()).
Freeze: tests/connect_retry.rs (std-only, imports oh_my_ib::ib::is_transient_io) — NEW spec-rev.
Live acceptance: omi --live account then immediately omi --live positions succeeds repeatedly.
Done when: arch.md + freeze/record + impl + PR; stage advances per CONTRACT; merge is human-confirmed.
On failure: attempts++; >=3 ⇒ blocked ⇒ pipeline-hunt.
<<< END

## seq=2 · 2026-06-28T15:36:38Z · arch→task · completed · by=claude-opus-4-8(claude-code)
done:   Locked design: is_transient_io(ErrorKind)->bool (WouldBlock|Interrupted|TimedOut) + bounded
        retry loop (3x, 250ms*attempt) in ib::connect, classifying via ibapi::Error::Io(io::Error).kind().
        Dead-port refused stays fast (no retry) so the existing frozen test is unaffected. No ADR.
output: .pipeline/connect-retry/arch.md, .pipeline/current.json
--- handoff ---
>>> NEXT (pipeline-task)
First: git pull --rebase. Read .pipeline/connect-retry/{PRD,arch}.md.
ONE card. Freeze tests/connect_retry.rs (std-only): use oh_my_ib::ib::is_transient_io; assert
  WouldBlock/Interrupted/TimedOut => true, ConnectionRefused/NotFound => false. Compiles only after impl
  adds the fn => RED. Two-commit freeze (test only = new spec-rev; then card 01 + current.json stage=task).
Card: spec-paths=[tests/connect_retry.rs], impl-paths=[src/ib/mod.rs], verify=[cargo build, cargo test --test connect_retry].
Gotcha: do NOT touch the other frozen specs (13e522d phase1, a072015 tz). New file = own spec-rev.
Done when: freeze+record on main; then pipeline-impl.
<<< END

## seq=3 · 2026-06-28T15:38:56Z · task→impl · completed · by=claude-opus-4-8(claude-code)
done:   Froze spec in two commits. Freeze fd72d90 (tests/connect_retry.rs only) = this feature's
        spec-rev; confirmed RED (unresolved oh_my_ib::ib::is_transient_io). Record: tasks/01.md
        (spec-rev=fd72d90), current.json stage=task.
output: .pipeline/connect-retry/tasks/01.md, .pipeline/current.json
--- handoff ---
>>> NEXT (pipeline-impl; roles.yaml impl=goal-driven-impl-claude)
First: git pull --rebase. Read .pipeline/connect-retry/{PRD,arch}.md + tasks/01.md.
  1. Branch feat/connect-retry from main.
  2. In src/ib/mod.rs add pub fn is_transient_io + the bounded retry loop in connect (per arch.md).
  3. Green: cargo build + cargo test --test connect_retry + full cargo test; clippy -D warnings; fmt
     (do NOT touch any tests/ spec file).
  4. LIVE ACCEPTANCE (gateway :4001): run `omi --live account` then immediately `omi --live positions`
     several times — must succeed with no surfaced EAGAIN. Confirm `omi health --port 65000` still fails fast.
  5. Push feat/connect-retry, open PR, flip card 01 review + stage=impl + journal on main.
Gotcha: new spec fd72d90 is this feature's gate only; 13e522d (phase1) + a072015 (tz) stay frozen/untouched.
Done when: card review, PR open, stage=impl; then pipeline-review (human-confirmed merge).
<<< END
