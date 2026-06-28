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

## seq=2 · 2026-06-28T15:12:44Z · arch→task · completed · by=claude-opus-4-8(claude-code)
done:   Locked the design: src/tz.rs (builtin_aliases + register_builtin_aliases via Once) wired into
        ib::connect before Client::connect; pub mod tz. ibapi::register_timezone_alias(impl Into<String>,
        impl Into<String>)->() confirmed at crate root, safe to call repeatedly. Curated unambiguous set:
        HKT/JST/KST/SGT. No ADR (small, additive, reversible).
output: .pipeline/tz-aliases/arch.md, .pipeline/current.json
--- handoff ---
>>> NEXT
Run pipeline-task on a FRESH session (rebuild from repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
First: git pull --rebase. Read .pipeline/tz-aliases/{PRD,arch}.md.
Your task:
  1. ONE card. Freeze tests/tz_aliases.rs (spec) — black-box-ish but imports the lib:
     `use oh_my_ib::tz;` assert builtin_aliases() contains ("HKT","Asia/Hong_Kong"), table non-empty,
     no empty entries, keys unique. Compiles only after impl adds the `tz` module => RED now.
  2. Two-commit freeze: freeze commit = tests/tz_aliases.rs only (the new spec-rev for THIS feature);
     record commit = tasks/01.md (spec-paths=[tests/tz_aliases.rs], impl-paths=[src/tz.rs, src/lib.rs,
     src/ib/mod.rs], spec-rev=<freeze sha>) + current.json{stage:task, full-verify=[cargo build, cargo test]}.
  3. Card verify: cargo build + cargo test --test tz_aliases.
Gotchas: do NOT touch the Phase 1 frozen tests (tests/cli_contract.rs, tests/data_commands.rs) — they
  stay frozen at their own spec-rev; this feature adds a NEW spec file with its OWN spec-rev.
Done when: freeze + record commits on main; then run pipeline-impl.
On failure: attempts++; >=3 ⇒ blocked ⇒ pipeline-hunt.
<<< END

## seq=3 · 2026-06-28T15:13:59Z · task→impl · completed · by=claude-opus-4-8(claude-code)
done:   Froze the spec in two commits. Freeze a072015 (tests/tz_aliases.rs only) = this feature's
        spec-rev; confirmed RED (unresolved import oh_my_ib::tz). Record commit: tasks/01.md
        (spec-rev=a072015), current.json{stage:task, full-verify=[cargo build, cargo test]}.
output: .pipeline/tz-aliases/tasks/01.md, .pipeline/current.json
--- handoff ---
>>> NEXT
Run pipeline-impl on a FRESH session (rebuild from repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
First: git pull --rebase. Read .pipeline/tz-aliases/{PRD,arch}.md + tasks/01.md.
roles.yaml: impl slot = goal-driven-impl-claude.
Your task (concrete, numbered):
  1. Branch feat/tz-aliases from main; flip card 01 in-progress on main (or do it in one continuous run).
  2. Add src/tz.rs (builtin_aliases + register_builtin_aliases via Once), pub mod tz in lib.rs, and call
     crate::tz::register_builtin_aliases() at the top of src/ib/mod.rs::connect.
  3. Make `cargo test --test tz_aliases` + `cargo build` green; never edit tests/ (any spec file).
  4. LIVE ACCEPTANCE (gateway up on :4001): `unset IBAPI_TIMEZONE_ALIASES; omi --live health` must
     connect (this is the whole point — proves finding A fixed). Also confirm Phase 1 cmds still work.
  5. Push feat/tz-aliases, open PR, flip card 01 review + stage=impl + journal on main.
Gotchas: only the new spec a072015 is this feature's gate; phase1-readonly's 13e522d tests stay frozen
  & untouched. Keep diff to src/tz.rs + lib.rs + ib/mod.rs (+ impl-owned unit tests if any).
Done when: card 01 review, PR open, stage=impl; then pipeline-review (human-confirmed merge).
On failure: attempts++; >=3 ⇒ blocked ⇒ pipeline-hunt.
<<< END
