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
