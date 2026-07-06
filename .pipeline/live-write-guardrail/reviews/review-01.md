# review-01 — live-write-guardrail card 01 (PR #27)

Verdict: ACCEPT + MERGED (squash `a843a08`). 2026-07-06. Semantic review: codex (GPT-5.5 xhigh); freeze
gate + full-suite gate + offline live acceptance + merge: cc; human-confirmed by operator.

## Deterministic freeze gate (cc) — PASS
`git diff 817c7d8 718d261 -- tests/live_write_guardrail.rs` = EMPTY (frozen spec untouched). spec-rev on
card = 817c7d8. Product diff (excluding `.pipeline/**`) = exactly `src/cli.rs` + `src/ib/mod.rs` +
`src/ib/trade.rs` (+95/-3) = the card's impl-paths. (The PR also carries `.pipeline/current.json` +
`journal.md` — impl-stage metadata travels with the PR, same as PR #26; not a code-scope violation.)

## Semantic review (codex) — ACCEPT (no findings)
Verified by reading + tests: the 4 pure seams match ADR 0030 (`trade.rs:195/200/208/223/247`);
`place_core` runs gate → posture → connect with correct `is_live`/`is_mkt`/`multiplier`/`notional` and
`AppError::config` mapping (`:545`); `option_combo` refuses live combo inside `!cfg.preview` before the
gate (`:852`); `require_live_write_gate` body unchanged, `place_with_client`/`option_close`/`cancel`
untouched (⇒ option-close stays EXEMPT), `shape_preview` unchanged; new seams contained to `trade.rs` +
the re-export + the frozen test (no read command imports them). codex ran `live_write_guardrail` 15/15 +
`order_preview_command` 12/12.

## Full-suite gate (cc) — PASS
`cargo test` = 257 tests, 0 failed (242 prior + 15 new). `cargo clippy --all-targets -- -D warnings`
clean. `order_preview_command` 12/12 (shape_preview JSON byte-identical). CodeRabbit CI = SUCCESS.

## Offline live acceptance (cc, Tiger :4001 CLOSED = paper mode — full safety) — PASS
All refuses fire BEFORE connect (exit 5, config), no order possible:
- `OMI_ALLOW_LIVE=1 omi --live buy AAPL 100 --limit 250` ⇒ "live notional 25000 exceeds cap 500 — raise OMI_MAX_NOTIONAL".
- `OMI_ALLOW_LIVE=1 omi --live buy AAPL 1` (MKT) ⇒ "live orders must be LMT — pass --limit".
- `OMI_ALLOW_LIVE=1 omi --live option-combo …` ⇒ "option-combo is paper-only during the trial".
- `OMI_ALLOW_LIVE=1 OMI_MAX_NOTIONAL=abc omi --live buy AAPL 1 --limit 250` ⇒ "invalid OMI_MAX_NOTIONAL 'abc'" (fail-closed).
Pass-through (guardrail Ok ⇒ reaches connect ⇒ exit 2, :4001 down):
- `OMI_MAX_NOTIONAL=100000 …buy AAPL 100 --limit 250` ⇒ connection error (the $25k order passed once the cap was raised).
- `OMI_MAX_NOTIONAL=25000 …buy AAPL 100 --limit 250` ⇒ connection error (boundary notional == cap passes; `>` not `>=`).

The within-cap → actually-place path is the operator's FIRST live trial order (needs `:4001` live) — not
asserted here (would place a real order).

## Arc
prd → arch (ADR 0030) → task (freeze 817c7d8) → impl (PR #27; network stall mid-wiring, resumed — seams
preserved) → codex ACCEPT → cc gates + offline acceptance → operator confirm → MERGED (a843a08).

## Follow-on
Live trial is UNBLOCKED. Next feature (operator-decided): trade log (append-only per-order black box).
