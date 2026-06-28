# journal — tz-aliases (Phase 1.1)

## seq=1 · 2026-06-28T15:11:02Z · prd→arch · completed · by=claude-opus-4-8(claude-code)
done:   New feature for follow-up A (review-05): omi must connect without IBAPI_TIMEZONE_ALIASES by
        auto-registering a curated unambiguous tz-alias set (HKT→Asia/Hong_Kong + JST/KST/SGT) before
        connect. Decision-complete PRD written; the gap was proven at Phase 1 live acceptance (Tiger
        gateway reports "HKT", rust-ibapi aborts the handshake). current.json repointed to tz-aliases.
output: .pipeline/tz-aliases/PRD.md, .pipeline/current.json
--- handoff ---
>>> NEXT
Run pipeline-arch on a FRESH session (rebuild from repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
First: git pull --rebase.
Read: .pipeline/tz-aliases/PRD.md; src/ib/mod.rs (connect); src/lib.rs; review-05.md (finding A).
Your task:
  1. arch.md: a `src/tz.rs` lib module (builtin_aliases() + register_builtin_aliases()) wired into
     ib::connect before Client::connect, guarded by std::sync::Once. pub mod tz in lib.rs.
  2. Confirm `ibapi::register_timezone_alias(&str, &str)` signature (crate root) and that calling it
     twice for the same alias is safe (else guard with Once — already planned).
Gotchas: only UNAMBIGUOUS abbreviations; env var IBAPI_TIMEZONE_ALIASES stays additive; do not touch
  the Phase 1 read-only contract; live acceptance = `unset IBAPI_TIMEZONE_ALIASES; omi --live health`.
Done when: arch.md committed, stage=arch, journal appended, then run pipeline-task.
On failure: attempts++; >=3 ⇒ blocked ⇒ pipeline-hunt.
<<< END
