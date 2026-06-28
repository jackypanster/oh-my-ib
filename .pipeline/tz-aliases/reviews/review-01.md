# review-01 - tz-aliases (Phase 1.1)

Verdict: approve; awaiting explicit human merge confirmation.

PR: https://github.com/jackypanster/oh-my-ib/pull/2
Head: 18d469442efb7a721e4fb71105ff206f8fa0647d
Review depth: quick (3 code files, +35 lines)
Scope: on target. The PR only adds built-in timezone alias registration and wires it before IB connect.

Findings:
- None blocking.

Source review:
- `src/tz.rs`: adds `builtin_aliases()` with HKT/JST/KST/SGT only, all mapped to unambiguous IANA zones.
- `src/tz.rs`: `register_builtin_aliases()` is process-idempotent via `std::sync::Once`.
- `src/ib/mod.rs`: alias registration runs before `Client::connect`, so the gateway handshake can use it.
- `src/lib.rs`: exports the `tz` module for the frozen offline spec.
- `ibapi` 3.1 source confirms the timezone registry is seeded from `IBAPI_TIMEZONE_ALIASES` on first access, then extended by `register_timezone_alias`; additive env aliases remain supported. Same-key override behavior is not part of this card's frozen/live acceptance.

Freeze gates:
- PASS: `git diff --exit-code a072015c641fd56de7a7f7721c8621bc967beba2 18d469442efb7a721e4fb71105ff206f8fa0647d -- tests/tz_aliases.rs`
- PASS: `git diff --exit-code 13e522dc70a432b0403cd75d4b5b82531a77a6fa 18d469442efb7a721e4fb71105ff206f8fa0647d -- tests/cli_contract.rs tests/data_commands.rs`

Verification on detached PR-head worktree `/tmp/oh-my-ib-pr2-review`:
- PASS: `cargo build`
- PASS: `cargo test` (12 unit + 5 cli_contract + 7 data_commands + 2 tz_aliases)
- PASS: `cargo clippy --all-targets -- -D warnings`

Live acceptance:
- Not re-run by this review session. Operator reports `IBAPI_TIMEZONE_ALIASES` unset, live gateway :4001: `omi --live health` and `omi --live account` both connect.

Merge gate:
- Do not merge until the operator gives explicit go.
- Before merging, re-read PR head and rerun the required gates if the head changed.
