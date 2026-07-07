# journal — sma-signal

## seq=1 · 2026-07-07T07:34:10Z · prd→arch · completed · by=cc/claude-opus-4-8
done:   Wrote PRD.md for sma-signal — a NEW read-only command `omi sma-signal [SYMBOL...]` reporting the
        Faber 200-day month-end timing signal (HOLD/EXIT) per symbol + drift context. Provenance: the
        strategy-lab bake-off proved 200SMA month-end timing is the one edge over buy-hold on the liquid
        panel. Two operator decisions locked via /think: read-only signal command (no write path, no gate)
        + month-end cadence (Faber, least whipsaw). Code-first survey confirmed: history.rs historical_data
        (Day/Trades/"2 Y") is directly reusable for bars; delayed data irrelevant (historical closes);
        read-command pattern (own module, no gate, default paper port) applies; positions() enables the
        no-args ergonomic. Pure sma_signal(bars,n) is the frozen heart.
output: .pipeline/sma-signal/PRD.md · .pipeline/current.json (stage=prd, feature=sma-signal)
--- handoff ---
>>> NEXT
Run pipeline-arch on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
Model: frontier SOTA required — operator assigns the bot; the pipeline can't verify the model.
First: git pull --rebase; no .env needed (offline arch work).
Read for context (before acting):
  - oh-my-ib/CLAUDE.md + AGENTS.md — repo conventions; ADR 0017 (write containment — NOTE: sma-signal is
    READ-ONLY so it does NOT apply); read-command pattern (quote.rs/history.rs/positions.rs).
  - .pipeline/sma-signal/PRD.md — what (problem, goal, locked D-decisions, the pure signal contract,
    scope, success criteria, gotchas, and a "For arch" section listing exactly what to resolve)
  - src/ib/history.rs (the historical_data call to reuse + the ibapi bar type — b.date/b.close),
    src/ib/positions.rs (positions() for no-args symbol resolution), src/ib/quote.rs (read-command shape),
    src/cli.rs (Command enum + read-args pattern), src/main.rs (dispatch), src/ib/mod.rs (re-exports)
Your task (concrete, numbered):
  1. grill-with-docs. Do NOT re-open the 2 locked decisions (read-only; month-end cadence). Resolve:
     (a) module placement — new READ module src/ib/signal.rs (pure sma_signal + gateway sma_signal_cmd),
     confirm NO gate / NOT trade.rs. (b) THE one real unknown: the exact ibapi bar `date` accessor to
     extract (year, month) for month-end grouping (history.rs only `{:?}`-formats it today — find the
     typed accessor). (c) how the gateway maps data.bars → Bar{ym, close} + factor/inline the fetch.
     (d) pin the Bar / SmaSignal types + sma_signal signature for the freeze. (e) no-args→positions()
     resolution + JSON output shape. Author ADR 0034 (read-only 200-SMA month-end signal + strategy-lab
     provenance + Phase-2 sma-tick deferral).
  2. Write .pipeline/sma-signal/arch.md (chosen shape + component boundaries + the exact Rust type
     signatures to freeze). Land CONTEXT.md + docs/adr/0034-*.md inline.
  3. Set current.json.stage=arch. Append seq=2 handoff to journal.md. git add arch.md + CONTEXT.md +
     docs/adr/0034-*.md + current.json + journal.md; commit once; push.
Feature gotchas (project-specific traps you MUST know):
  - sma-signal is READ-ONLY: NO --live/OMI_ALLOW_LIVE gate, default paper port; ADR 0017 containment is
    irrelevant (no writes). Do not add write-path machinery.
  - Month-end = last TRADING day of the month in the bar series (group by year,month, take last).
  - Use the last COMPLETED month-end (exclude the in-progress final month) — the standing signal.
  - SMA computed AS OF the month-end (Faber), and ALSO at the latest bar (drift) — report both.
  - The ibapi bar date accessor for (year,month) is the one thing to verify against the crate.
  - Delayed data is fine (historical closes). Insufficient history (<n bars) ⇒ state=INSUFFICIENT, no crash.
Done when: arch.md + CONTEXT.md + ADR 0034 committed & pushed; Bar/SmaSignal + sma_signal signature pinned
precisely enough for pipeline-task to write red tests without re-deciding.
On success: stage→arch, then run pipeline-task (decompose into atomic card(s), write a red test).
On failure: attempts++; >=3 ⇒ blocked ⇒ run pipeline-hunt.
<<< END

## seq=2 · 2026-07-07T07:38:38Z · arch→task · completed · by=cc/claude-opus-4-8
done:   grill-with-docs resolved all arch items. THE unknown resolved: ibapi historical Bar.date is
        `BarTimestamp` (enum Date(time::Date)|DateTime(OffsetDateTime), crate 3.1 historical/mod.rs:164);
        (year,month) via time's .year()/.month() — the gateway strips it to (i32,u32) so the pure fn stays
        ibapi-free/offline. Module: new READ module src/ib/signal.rs (pure sma_signal frozen + gateway
        sma_signal_cmd review-by-reading), NO gate / NOT trade.rs (read-only, ADR 0017 N/A). arch.md pins
        the exact types (Bar/SmaSignal/SignalState + sma_signal sig) + the algorithm; ADR 0034 + CONTEXT.md
        written. Add `time` as a direct Cargo dep (already transitive via ibapi).
output: .pipeline/sma-signal/arch.md · CONTEXT.md · docs/adr/0034-sma-signal.md · current.json (stage=arch)
--- handoff ---
>>> NEXT
Run pipeline-task on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
Model: frontier SOTA required — operator assigns the bot.
First: git pull --rebase; no .env needed (offline task work).
Read for context (before acting):
  - oh-my-ib/CLAUDE.md + AGENTS.md — conventions; two-commit freeze protocol; read-command pattern.
  - .pipeline/sma-signal/arch.md — the EXACT Rust types to freeze + the algorithm + "For task" section.
  - .pipeline/sma-signal/docs/adr/0034-sma-signal.md — binding decisions + Freeze coverage.
  - .pipeline/sma-signal/PRD.md + CONTEXT.md — problem/goal/glossary.
  - tests/grid_tick.rs — the prior frozen-test style (approx helper for clippy float_cmp; import from
    oh_my_ib::…) to MIRROR. src/ib/history.rs (historical_data reuse), src/ib/positions.rs.
Your task (concrete, numbered):
  1. Write RED tests/sma_signal.rs (spec-paths), importing oh_my_ib::ib::{sma_signal, Bar, SmaSignal,
     SignalState}. Cover ADR 0034 Freeze coverage: (a) a bar series whose last completed month-end close is
     ABOVE the n-SMA ⇒ state==Hold; BELOW ⇒ Exit; (b) distance_pct = (close-sma)/sma*100 (approx); (c)
     last-COMPLETED-month-end selection — build bars spanning ≥2 months where the in-progress final month
     differs from the prior month-end, assert as_of_month_end is the PRIOR month and sma is computed as of
     it (not the latest bar); (d) < n bars ⇒ state==Insufficient (no panic); (e) latest_* drift fields
     reflect the final bar. Use a small n (e.g. n=3) with hand-built Bar arrays so the SMA is exact and
     hand-checkable. Mirror grid_tick.rs's `approx` helper (NEVER == on f64 — clippy float_cmp under -D
     warnings). RED via the unresolved oh_my_ib::ib::sma_signal import — do NOT add any src/ stub.
  2. Freeze commit = ONLY tests/sma_signal.rs (compile-fail on the import). Its sha = spec-rev.
  3. Record commit = tasks/01.md (ONE card: frozen pure sma_signal; gateway+wiring review-by-reading) with
     frontmatter status=todo attempts=0 verify=[cargo build, cargo test --test sma_signal]
     spec-paths=[tests/sma_signal.rs] impl-paths=[src/ib/signal.rs, src/ib/mod.rs, src/cli.rs, src/main.rs,
     Cargo.toml] spec-rev=<step-2 sha>. Advance current.json.stage=task. Append seq=3 handoff. Commit
     metadata (never spec-paths). Push both commits.
Feature gotchas (project-specific traps you MUST know):
  - Pure fn takes Bar{ym:(i32,u32),close} — NO ibapi in the test. Freeze offline with tiny hand-built arrays.
  - n=3 test tip: for last-completed-month-end, e.g. bars over month A (several) then month B (the
    in-progress final month, fewer bars) — as_of must be A's last bar, sma = mean of the 3 closes ending at
    A's month-end, NOT including B.
  - clippy float_cmp bites the test under --all-targets -D warnings → approx helper (mirror grid_tick.rs).
  - Insufficient: <n bars ⇒ state=Insufficient, numeric fields 0.0, no slice panic.
  - This is READ-ONLY: the card's out-of-scope must forbid any gate/write symbols in signal.rs.
Done when: freeze commit (RED, compile-fail on import) + record commit (card + stage=task) pushed; spec-rev
on the card. On success: stage→task, then run pipeline-impl (omp / capable-local OK) on feat/sma-signal.
On failure: attempts++; >=3 ⇒ blocked ⇒ run pipeline-hunt.
<<< END

## seq=3 · 2026-07-07T07:42:27Z · task→impl · completed · by=cc/claude-opus-4-8
done:   Two-commit freeze. FREEZE commit 58f31d4 = tests/sma_signal.rs ONLY (5 tests, n=3 hand-built Bar
        arrays; RED via unresolved oh_my_ib::ib::sma_signal, verified single E0432, no src/ stub; approx
        helper → clippy float_cmp-clean once green). RECORD commit (this) = tasks/01.md + current.json
        (stage=task). ONE card: frozen pure sma_signal; gateway sma_signal_cmd + wiring review-by-reading.
        spec-rev 58f31d4 on the card.
output: .pipeline/sma-signal/tasks/01.md · current.json (stage=task) · spec-rev=58f31d4
--- handoff ---
>>> NEXT
Run pipeline-impl on a FRESH session (omp / goal-driven-impl-claude). Make tests/sma_signal.rs GREEN.
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
Model: capable-local OK (impl).
First: git pull --rebase; no .env needed for build/test.
Read for context (before acting):
  - oh-my-ib/CLAUDE.md + AGENTS.md — conventions; freeze gate (never touch tests/); read-command pattern.
  - .pipeline/sma-signal/tasks/01.md — THE CARD: exact types, the sma_signal algorithm pseudo, the gateway
    steps (§1–§5), out-of-scope, freeze coverage. Follow closely.
  - .pipeline/sma-signal/arch.md + docs/adr/0034-sma-signal.md + CONTEXT.md — boundaries + decisions.
  - tests/sma_signal.rs — frozen spec (DO NOT edit; spec-rev 58f31d4).
  - src/ib/history.rs (historical_data call + ibapi BarSize/BarTimestamp/WhatToShow/ToDuration imports to
    mirror), src/ib/positions.rs (positions() for no-args symbols), src/ib/quote.rs (read-command shape),
    src/cli.rs (Command enum + read args), src/main.rs (dispatch), src/ib/mod.rs (re-exports), Cargo.toml.
Your task (concrete, numbered):
  1. Cut feat/sma-signal from trunk (inherits spec-rev 58f31d4).
  2. Implement per tasks/01.md §1–§5: src/ib/signal.rs (pure sma_signal + SignalState/Bar/SmaSignal +
     gateway sma_signal_cmd + ym_of BarTimestamp→(i32,u32) helper + JSON shape); mod.rs mod+re-export;
     cli.rs SmaSignal command; main.rs dispatch; Cargo.toml add `time` direct dep.
  3. Verify: cargo test --test sma_signal GREEN; cargo build; cargo clippy --all-targets -- -D warnings
     clean (no == on f64 in any new code); ALL prior suites GREEN and byte-identical; tests/sma_signal.rs
     untouched (freeze gate).
  4. Open PR from feat/sma-signal → set current.json.pr; append seq=4 handoff; push.
Feature gotchas (project-specific traps you MUST know):
  - READ-ONLY: NO gate / NO write symbols (place_order/cancel_order/require_live_write_gate) in signal.rs.
  - Month-end = last bar of each (year,month) group; as_of = last month STRICTLY before the in-progress
    final month (else final bar if only one month). SMA computed AS OF as_of, not just latest.
  - Pure fn is ibapi-free (takes Bar{ym,close}); the gateway does the BarTimestamp strip via ym_of.
  - Insufficient: empty/ <n / not-enough-up-to-month-end ⇒ state=Insufficient, numeric 0.0, no slice panic.
  - clippy float_cmp: mirror grid_tick.rs's approach; the pure fn already avoids float == (uses >=).
  - `time` is transitive via ibapi — add it directly so BarTimestamp inner accessors compile cleanly.
Done when: tests/sma_signal.rs GREEN + build/clippy clean + prior suites green + PR opened.
On success: status→review (card), then run pipeline-review (codex / check): freeze gate (git diff 58f31d4
<branch-tip> -- tests/sma_signal.rs EMPTY) + full-suite green + read-only grep + paper acceptance + human-confirm merge.
On failure: attempts++; >=3 ⇒ blocked ⇒ run pipeline-hunt.
<<< END

## seq=4 · 2026-07-07 · task→impl · completed · by=omp/π (GLM)
done:   Implemented card 01 per tasks/01.md §1–§5. src/ib/signal.rs = SignalState/Bar/SmaSignal
        types + pure sma_signal (Faber rule: as_of_idx = last bar whose month < the in-progress
        final month, else the final bar; HOLD if month-end close >= n-day SMA, else EXIT;
        INSUFFICIENT on empty/<n/not-enough-up-to-month-end; numeric 0.0, never panics; latest_*
        drift reflects final bar) + gateway sma_signal_cmd (resolve symbols — positions fallback
        when none given; fetch 2Y daily bars via historical_data; ym_of strips BarTimestamp to
        (year,month); JSON envelope) + held_symbols helper. mod.rs re-exports; cli.rs
        SmaSignal(SmaSignalArgs{symbols, --sma 200}); main.rs dispatch; Cargo.toml adds time=0.3
        (transitive via ibapi, version-matched 0.3.51).
posture: READ-ONLY — signal.rs has NO require_live_write_gate / place_order / cancel_order
        (review greps). ADR 0017 does not apply. Default paper port.
output: PR #31 (feat/sma-signal → main), commit b2a040b
verify: cargo build OK · cargo test --test sma_signal 5/5 GREEN · cargo clippy --all-targets
        -D warnings CLEAN (no float_cmp — pure fn uses only >= on floats) · cargo test (full
        suite) all-green, 0 failures (all prior suites byte-identical).
--- handoff ---
>>> NEXT
Run pipeline-review on a FRESH session (only card is 01, now status=review ⇒ feature complete).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=31
Operator: review = codex (pipeline-review). Merge gated on explicit human confirm (CONTRACT).
Read for context (before acting):
  - .pipeline/sma-signal/tasks/01.md — the card (impl-paths, out-of-scope).
  - tests/sma_signal.rs — the frozen spec (spec-rev 58f31d4); freeze gate diffs spec-paths.
  - PR #31 diff — 6 files +245 -0.
Review checks: freeze gate (git diff 58f31d4 <branch-tip> -- tests/sma_signal.rs must be EMPTY);
read-only grep (signal.rs ⇒ no require_live_write_gate / place_order / cancel_order); full-suite
gate; review-by-reading the gateway (symbol resolution incl. no-args→positions, historical_data
fetch, ym_of strip, JSON shape).
OPERATOR ACCEPTANCE (post-merge, paper :4002): omi sma-signal NVDA MU QQQ → HOLD/EXIT per name;
omi sma-signal (no args) → signals held positions.
<<< END
