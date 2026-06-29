# review-01 - connect-retry (review-05 follow-up B)

Verdict: approve; awaiting explicit human merge confirmation.

PR: https://github.com/jackypanster/oh-my-ib/pull/3
Head: 7959d229f641a11642556c49d6ec8b6e197bc43b
Review depth: quick (1 code file, +39/-4)
Scope: on target. The PR only changes `src/ib/mod.rs` to classify transient IO errors and retry `connect`.

Findings:
- None blocking.

Source review:
- `src/ib/mod.rs:29-41`: `is_transient_io` classifies only `WouldBlock`, `Interrupted`, and `TimedOut` as retryable; `ConnectionRefused` stays permanent.
- `src/ib/mod.rs:48-70`: `connect` still registers timezone aliases before connecting, then retries only `ibapi::Error::Io(e)` when `is_transient_io(e.kind())` is true.
- `src/ib/mod.rs:29-61`: retry bound is initial attempt plus 3 retries with 250ms, 500ms, 750ms sleeps; worst-case added latency is 1.5s, only on transient errors.
- `ibapi` 3.1 source confirms `ibapi::Error::Io(std::io::Error)` exists, so the implementation classifies by structured `ErrorKind`, not by localized/string error text.
- Pattern sweep found the single `Client::connect` path in this crate; routing all read commands through `ib::connect` keeps the change centralized.

Freeze gates:
- PASS: `git diff --exit-code fd72d903e2abfb435e0e853697adb223bc6dcf22 7959d229f641a11642556c49d6ec8b6e197bc43b -- tests/connect_retry.rs`
- PASS: `git diff --exit-code a072015c641fd56de7a7f7721c8621bc967beba2 7959d229f641a11642556c49d6ec8b6e197bc43b -- tests/tz_aliases.rs`
- PASS: `git diff --exit-code 13e522dc70a432b0403cd75d4b5b82531a77a6fa 7959d229f641a11642556c49d6ec8b6e197bc43b -- tests/cli_contract.rs tests/data_commands.rs`

Verification on detached PR-head worktree `/tmp/oh-my-ib-pr3-review-7959d22`:
- PASS: `cargo build`
- PASS: `cargo test` (12 unit + 5 cli_contract + 2 connect_retry + 7 data_commands + 2 tz_aliases)
- PASS: `cargo clippy --all-targets -- -D warnings`
- PASS: `./target/debug/omi health --port 65000 --format json` returned exit 2 in 0.023s with `{"error":{"code":"connection",...}}`, confirming refused fails fast.

Live acceptance:
- Not re-run by this review session. Operator reports live gateway :4001: `omi --live account` immediately followed by `omi --live positions`, 4/4 rounds clean, no surfaced EAGAIN.

Doc debt:
- None. The retry behavior is captured in `PRD.md`, `arch.md`, the frozen card, and this review record.

Merge gate:
- Do not merge until the operator gives explicit go.
- Before merging, re-read PR head and rerun required gates if the head changed.
