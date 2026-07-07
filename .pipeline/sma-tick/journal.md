# journal — sma-tick

## seq=1 · 2026-07-07T12:00:44Z · prd→arch · completed · by=cc/claude-opus-4-8
done:   Wrote PRD.md for sma-tick — the active counterpart to sma-signal: each month reconcile the QQQ
        position to the 200-day month-end signal (HOLD ⇒ target lot=10 shares, EXIT ⇒ 0). WRITE feature
        (real orders), grid-tick shape. Four decisions locked via /think: binary target (not ladder);
        QQQ-only + lot=10 + flags (no toml); paper-only v1 (10 QQQ ≈ $7.2k ≫ $500 cap); pure
        plan_sma_tick frozen. Code-first survey confirmed all reuse: sma_signal (PR #31), place_with_client
        + build_stk_order (grid-tick, pub(crate)), positions(), LIVE_PORT. Almost no new write code.
output: .pipeline/sma-tick/PRD.md · .pipeline/current.json (stage=prd, feature=sma-tick)
--- handoff ---
>>> NEXT
Run pipeline-arch on a FRESH session (rebuild from repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
Model: frontier SOTA required.
First: git pull --rebase.
Read for context:
  - oh-my-ib/CLAUDE.md + AGENTS.md — ADR 0017 write-containment; grid-tick as the write-reconcile precedent.
  - .pipeline/sma-tick/PRD.md — locked decisions, the pure plan_sma_tick contract, scope, gotchas, For-arch.
  - .pipeline/grid-tick/docs/adr/0033-grid-tick.md — the write-orchestration containment pattern to mirror.
  - .pipeline/sma-signal/docs/adr/0034-sma-signal.md — the signal it reuses.
  - src/ib/trade.rs (place_with_client :470 pub(crate), build_stk_order), src/ib/signal.rs (sma_signal +
    the bar fetch to share/re-call), src/ib/positions.rs (positions()), src/config.rs (LIVE_PORT),
    src/ib/grid.rs (the grid-tick driver pattern to mirror), src/cli.rs, src/main.rs, src/ib/mod.rs.
Your task (numbered):
  1. grill-with-docs. Do NOT re-open the 4 locked decisions. Resolve: (a) module = new src/ib/sma_tick.rs
     (pure plan_sma_tick + gateway sma_tick_cmd); confirm paper-only guard + containment (no raw
     place_order — compose build_stk_order + place_with_client). (b) THE order-type decision: MKT DAY
     (accept [399] queued-to-open as SUBMITTED, not error — check how omi buy currently treats [399]) vs
     marketable LMT at latest close. (c) how the gateway computes the signal (share a bar-fetch helper
     with sma_signal_cmd, or re-call historical_data) + reads current qty from positions(). (d) pin
     plan_sma_tick + TickAction for the freeze. (e) JSON shape + --dry-run. Author ADR 0035.
  2. Write arch.md + CONTEXT.md + docs/adr/0035-*.md. Set current.json.stage=arch. Append seq=2 handoff.
     Commit once; push.
Feature gotchas:
  - WRITE feature: paper-only (refuse LIVE_PORT before connect); compose trade.rs choke points (ADR 0017).
  - Binary target (lot or 0), never accumulate. plan_sma_tick pure/frozen; gateway review-by-reading.
  - [399] MKT-queued-to-open should be a SUCCESS for a monthly timing order (matches Faber act-at-open).
  - Reuse sma_signal (pure) + place_with_client; don't reimplement.
Done when: arch.md + CONTEXT.md + ADR 0035 pushed; plan_sma_tick/TickAction pinned for task.
On success: stage→arch, run pipeline-task. On failure: attempts++; >=3 ⇒ blocked ⇒ pipeline-hunt.
<<< END

## seq=2 · 2026-07-07T12:05:30Z · arch→task · completed · by=cc/claude-opus-4-8
done:   grill-with-docs resolved arch. D-ORDER resolved to MARKETABLE LMT (not MKT): live evidence shows
        LMT→clean PreSubmitted ack, MKT-when-closed→[399] error; marketable LMT (Buy latest_close×1.02 /
        Sell ×0.98) fills at open, rests clean when closed, reuses place_with_client UNCHANGED, live-shaped.
        Module = new WRITE-orchestration src/ib/sma_tick.rs (pure plan_sma_tick frozen + gateway), paper-
        only, composes trade.rs choke points (ADR 0017). signal_for extracted pub(crate) from
        sma_signal_cmd (shared fetch, sma-signal byte-identical). arch.md pins plan_sma_tick + TickAction;
        ADR 0035 + CONTEXT.md written.
output: .pipeline/sma-tick/arch.md · CONTEXT.md · docs/adr/0035-sma-tick.md · current.json (stage=arch)
--- handoff ---
>>> NEXT
Run pipeline-task on a FRESH session (rebuild from repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
Model: frontier SOTA required.
First: git pull --rebase.
Read for context:
  - oh-my-ib/CLAUDE.md + AGENTS.md — two-commit freeze protocol.
  - .pipeline/sma-tick/arch.md — EXACT types (plan_sma_tick + TickAction) + algorithm + "For task".
  - .pipeline/sma-tick/docs/adr/0035-sma-tick.md — binding decisions + Freeze coverage.
  - tests/sma_signal.rs / tests/grid_tick.rs — prior frozen-test style (approx helper; import from oh_my_ib::ib).
Your task (numbered):
  1. Write RED tests/sma_tick.rs (spec-paths), importing oh_my_ib::ib::{plan_sma_tick, TickAction,
     SignalState}. Cover ADR 0035 Freeze coverage: Hold+0+lot10⇒Buy 10; Hold+10⇒Noop; Hold+4⇒Buy 6;
     Hold+15⇒Sell 5; Exit+10⇒Sell 10; Exit+0⇒Noop; Insufficient⇒Noop. approx helper (clippy float_cmp;
     match TickAction and approx the qty). RED via unresolved oh_my_ib::ib::plan_sma_tick — NO src/ stub.
  2. Freeze commit = ONLY tests/sma_tick.rs. Its sha = spec-rev.
  3. Record commit = tasks/01.md (ONE card: frozen pure plan_sma_tick; gateway + signal_for extraction +
     wiring review-by-reading). Frontmatter: status=todo attempts=0 verify=[cargo build, cargo test --test
     sma_tick] spec-paths=[tests/sma_tick.rs] impl-paths=[src/ib/sma_tick.rs, src/ib/signal.rs,
     src/ib/mod.rs, src/cli.rs, src/main.rs] spec-rev=<step-2 sha>. Advance current.json.stage=task.
     Append seq=3. Commit metadata (never spec-paths). Push both commits.
Feature gotchas:
  - WRITE feature paper-only; composes trade.rs choke points (no raw place_order in sma_tick.rs).
  - Binary target (lot or 0). plan_sma_tick pure/frozen; f64, no == on floats (uses >/<).
  - Marketable LMT (latest_close ×1.02/×0.98) — order type is review-by-reading, NOT in the frozen fn.
  - signal_for extraction must keep sma_signal_cmd/sma-signal byte-identical (frozen tests stay green).
Done when: freeze commit (RED) + record commit (card + stage=task) pushed; spec-rev on the card.
On success: stage→task, run pipeline-impl (omp). On failure: attempts++; >=3 ⇒ blocked ⇒ pipeline-hunt.
<<< END

## seq=3 · 2026-07-07T12:10:37Z · task→impl · completed · by=cc/claude-opus-4-8
done:   Two-commit freeze. FREEZE commit 22b1a9e = tests/sma_tick.rs ONLY (7 tests; RED via unresolved
        oh_my_ib::ib::plan_sma_tick, verified single E0432, no src/ stub; approx helper). RECORD commit
        (this) = tasks/01.md + current.json (stage=task). ONE card: frozen pure plan_sma_tick; gateway +
        signal_for extraction + wiring review-by-reading. spec-rev 22b1a9e.
output: .pipeline/sma-tick/tasks/01.md · current.json (stage=task) · spec-rev=22b1a9e
--- handoff ---
>>> NEXT
Run pipeline-impl on a FRESH session (omp / goal-driven-impl-claude). Make tests/sma_tick.rs GREEN.
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
Model: capable-local OK.
First: git pull --rebase.
Read: oh-my-ib/CLAUDE.md + AGENTS.md; .pipeline/sma-tick/tasks/01.md (THE CARD — exact types, gateway
§1–§5, algorithm, out-of-scope); arch.md + ADR 0035 + CONTEXT.md; tests/sma_tick.rs (frozen, spec-rev
22b1a9e); src/ib/grid.rs (mirror the write-reconcile driver); src/ib/signal.rs (extract signal_for);
src/ib/trade.rs (place_with_client :470, build_stk_order); src/ib/positions.rs; src/config.rs (LIVE_PORT).
Your task:
  1. Cut feat/sma-tick from trunk (inherits spec-rev 22b1a9e).
  2. Implement per tasks/01.md §1–§5: src/ib/sma_tick.rs (pure plan_sma_tick + TickAction + gateway
     sma_tick_cmd, paper-only, marketable LMT via build_stk_order + place_with_client); extract
     pub(crate) signal_for in signal.rs (sma_signal_cmd delegates, byte-identical); mod.rs re-export;
     cli.rs SmaTick; main.rs dispatch.
  3. Verify: cargo test --test sma_tick GREEN; cargo build; cargo clippy --all-targets -- -D warnings;
     ALL prior suites (sma_signal, grid_tick, 4 write suites) GREEN + byte-identical; tests/ untouched.
  4. Open PR from feat/sma-tick → set current.json.pr; append seq=4; push.
Feature gotchas:
  - Paper-only (refuse LIVE_PORT before connect). NO raw place_order in sma_tick.rs (compose choke points).
  - Binary target; plan_sma_tick pure/frozen; no == on f64 (clippy float_cmp).
  - Marketable LMT (latest_close ×1.02 buy / ×0.98 sell) — reuse place_with_client unchanged; NOT MKT.
  - signal_for extraction MUST keep sma_signal_cmd + tests/sma_signal.rs byte-identical/green.
Done when: tests/sma_tick.rs GREEN + build/clippy clean + prior suites green + PR opened.
On success: status→review, run pipeline-review (codex): freeze gate (git diff 22b1a9e <tip> --
tests/sma_tick.rs EMPTY) + full-suite + paper-only grep + containment grep + paper acceptance.
On failure: attempts++; >=3 ⇒ blocked ⇒ pipeline-hunt.
<<< END

## seq=4 · 2026-07-07 · task→impl · completed · by=omp/π (GLM)
done:   Implemented card 01 per tasks/01.md §1–§5. src/ib/sma_tick.rs = TickAction{Buy,Sell,Noop} +
        pure plan_sma_tick (binary target: HOLD⇒lot, EXIT⇒0, INSUFFICIENT⇒Noop; delta=target-
        current_qty ⇒ Buy(delta)/Sell(-delta)/Noop; uses >/< vs 1e-9, no f64 ==) + gateway sma_tick_cmd
        (paper-only guard LIVE_PORT⇒config/exit5 offline pre-connect; --sma>=1 guard; connect;
        signal_for; positions read for current qty; plan; --dry-run returns signal+plan no order; else
        ONE marketable LMT Buy@round2(latest_close*1.02)/Sell@round2(latest_close*0.98) via
        build_stk_order + place_with_client, not MKT). signal.rs: extracted pub(crate) signal_for
        (fetch-2Y-Day + map-to-Bar + sma_signal); sma_signal_cmd delegates (byte-identical,
        tests/sma_signal.rs GREEN). mod.rs re-exports; cli.rs SmaTick(SmaTickArgs{symbol default QQQ,
        --lot 10.0, --sma 200, --dry_run}); main.rs dispatch.
containment: NO raw place_order/cancel_order in src/ib/sma_tick.rs — composes build_stk_order +
        place_with_client only (ADR 0017 holds; review greps). Passes resolved &account to
        place_with_client (ADR 0024 stamp; no double-stamp). Paper-only: LIVE_PORT refused offline.
output: PR #32 (feat/sma-tick → main), commit 3409b0c
verify: cargo build OK · cargo test --test sma_tick 7/7 GREEN · cargo clippy --all-targets
        -D warnings CLEAN (no float_cmp — pure fn uses >/< vs 1e-9) · cargo test (full suite)
        all-green, 0 failures (sma_signal + grid_tick + 4 prior write suites green + byte-identical;
        signal_for extraction left sma-signal byte-identical).
--- handoff ---
>>> NEXT
Run pipeline-review on a FRESH session (only card is 01, now status=review ⇒ feature complete).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=32
Operator: review = codex (pipeline-review). Merge gated on explicit human confirm (CONTRACT).
Read for context (before acting):
  - .pipeline/sma-tick/tasks/01.md — the card (impl-paths, out-of-scope byte-identical list).
  - tests/sma_tick.rs — frozen spec (spec-rev 22b1a9e); freeze gate diffs spec-paths.
  - PR #32 diff — 5 files +217 -16.
Review checks: freeze gate (git diff 22b1a9e <branch-tip> -- tests/sma_tick.rs must be EMPTY);
containment grep (sma_tick.rs ⇒ no place_order/cancel_order); paper-only guard (LIVE_PORT ⇒ config
offline); signal_for extraction left sma-signal byte-identical (tests/sma_signal.rs GREEN); marketable
LMT not MKT (Buy@*1.02, Sell@*0.98, round2); full-suite gate.
OPERATOR ACCEPTANCE (post-merge, paper :4002): omi sma-tick QQQ --lot 10 --dry-run shows signal +
current/target/action; a real run places the reconcile LMT; omi --live sma-tick QQQ refused paper-only.
<<< END

## seq=5 · 2026-07-07 · impl · note · by=omp/π (GLM)
done:   Operator decision: passive default symbol QQQ → QQQM (Nasdaq-100 same-index, lower expense
        ratio). Two default-value edits on feat/sma-tick:
        - src/ib/sma_tick.rs:78  unwrap_or_else(|| "QQQ".into()) → "QQQM".into()
        - src/cli.rs:307         doc "default QQQ" → "default QQQM"
        Pure default change; plan_sma_tick frozen test is symbol-agnostic (unaffected); callers passing
        an explicit symbol see no behavior change.
output: PR #32 (feat/sma-tick) update commit 9a71bfb (c34c9b5..9a71bfb)
verify: cargo build OK · cargo test (full suite) all-green, 0 failures · cargo clippy --all-targets
        -D warnings CLEAN. tests/ untouched (freeze gate empty). Card remains status=review.
<<< END

## seq=6 · 2026-07-07T12:30:25Z · review→impl · failed · by=codex
done:   Review verdict written: REJECT for card 01. Freeze gate passed, full verify passed, containment
        grep passed, paper-only guard passed, and default QQQM was confirmed. Semantic review found two
        write-path fail-closed issues: position-read errors are treated as flat, and `--lot` is not
        validated as finite/positive before it can drive a placement.
output: .pipeline/sma-tick/reviews/review-01.md · .pipeline/sma-tick/tasks/01.md
--- handoff ---
>>> NEXT
Run pipeline-impl on a FRESH session (assume you know nothing -- rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=32 card=01
Model: capable-local OK (impl) -- operator assigns the bot.
First: git pull --rebase. Reuse the existing feat/sma-tick branch / PR #32.
Read for context:
  - oh-my-ib/AGENTS.md + CLAUDE.md -- repo conventions; write safety and structured errors.
  - .pipeline/sma-tick/tasks/01.md -- card 01, now attempts=1/status=todo, plus Review rejection 01.
  - .pipeline/sma-tick/reviews/review-01.md -- blocking findings and verification already run.
  - .pipeline/sma-tick/docs/adr/0035-sma-tick.md + arch.md + CONTEXT.md -- binding design.
  - tests/sma_tick.rs -- frozen spec; DO NOT edit (spec-rev 22b1a9e).
Your task:
  1. Fix position-read fail-closed behavior. `current_position_qty` must not swallow `positions(cfg)`
     errors. Return `Result<f64, AppError>` and propagate, or read positions through the already-resolved
     account/client so signal, position, and placement share one account authority. Only absent symbol in
     a successful positions payload may map to 0.0.
  2. Validate `--lot` before any gateway work: finite and > 0.0, else structured usage/config error with
     context `sma-tick`. Cover negative, zero, and non-finite inputs in implementation-owned tests or CLI
     tests without touching `tests/sma_tick.rs`.
  3. Preserve ADR 0017 containment: no raw `place_order` / `cancel_order` in `src/ib/sma_tick.rs`; keep
     marketable LMT pricing and QQQM default unchanged.
  4. Verify on `feat/sma-tick`: `cargo build`; `cargo test --test sma_tick`; `cargo test`;
     `cargo clippy --all-targets -- -D warnings`. Re-run freeze gate:
     `git diff 22b1a9e origin/feat/sma-tick -- tests/sma_tick.rs` must be empty.
Feature gotchas:
  - The branch currently contains expected `.pipeline` metadata skew from the QQQM default sync commit.
    Do not "fix" frozen tests or trunk metadata in implementation; keep product-code changes scoped.
  - This is a paper-only write command. Unknown position state must fail closed, not plan from 0.
Done when: PR #32 is updated, card 01 is back at review, full verify is green, and review can rerun.
On failure: attempts++; attempts >= 3 => blocked => run pipeline-hunt.
<<< END
