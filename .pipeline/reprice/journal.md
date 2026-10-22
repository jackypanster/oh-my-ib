# journal — reprice

## seq=1 · 2026-10-22T05:40:00Z · ∅→prd · completed · by=cloud-agent
done:   Operator goal locked: `omi reprice --price P ORDER_ID`, preview unless `--confirm`,
        notional re-check, live gates kept, ftnn exits 4/7/8/64 on writes, no cancel+replace.
        ibapi 3.1 `place_order` doc read in the registry: "Places or modifies an Order".
        No separate modify API. Tiger not observable from this VM.
output: .pipeline/reprice/PRD.md
--- handoff ---
>>> NEXT
arch: pin the pure seam so preview / same-id submit / guard 64 / unconfirmed 8 are offline.
Do not change build_stk_order arity, shape_order_ack, or AppError::timeout exit 6.
<<< END

## seq=2 · 2026-10-22T05:50:00Z · prd→arch · completed · by=cloud-agent
done:   Seam is plan_reprice + reprice_order + classify_submit_error + submit_timeout_error.
        Preview never returns Submit. Confirm returns the existing order id. Cap block is
        guard/64 before place_order. Ack timeout is unconfirmed/8. ADR 0040 amends ADR 0017
        "no modify" and the write-timeout exit only.
output: .pipeline/reprice/arch.md, CONTEXT.md, docs/adr/0040-reprice.md
--- handoff ---
>>> NEXT
task: one card. spec-paths tests/reprice.rs + help inventory re-freeze ("reprice" only).
<<< END

## seq=3 · 2026-10-22T06:00:00Z · arch→task · completed · by=cloud-agent
done:   One card (01). Spec and impl are one change: the operator asked for a green PR,
        not a red freeze commit. help_command.rs gains "reprice" and nothing else.
        No other frozen spec is edited.
output: .pipeline/reprice/tasks/01.md, tests/reprice.rs
--- handoff ---
>>> NEXT
impl is in the same change (src/ib/trade.rs containment). full-verify:
cargo test --offline --locked && cargo clippy --all-targets -- -D warnings.
<<< END

## seq=4 · 2026-10-22T06:20:00Z · task→impl · completed · by=cloud-agent
done:   `omi reprice` preview/confirm, same-id place_order, guard/64, unconfirmed/8,
        rejected/7, data/4. Existing write success JSON unchanged. Read timeout stays 6.
        Live env gate stays config/5. AGENTS.md updated (modify is in scope; exit table).
        No gateway was contacted. Tiger modify remains UNVERIFIED — paper probe is in
        docs/write-path-semantics.md and the PR. Do not merge.
output: src/ib/trade.rs, src/cli.rs, src/main.rs, src/surface.rs, src/error.rs, src/ib/mod.rs,
        AGENTS.md, CLAUDE.md, docs/write-path-semantics.md, tests/reprice.rs, tests/help_command.rs
--- handoff ---
>>> NEXT
Review is the PR against main. Human confirms. Do not merge from this run.
Paper acceptance (operator, :4002, no live): place a far LMT, preview reprice (price unchanged),
confirm reprice (same order id, new price, no second order), cancel. If Tiger rejects or
duplicates, record it — do not add cancel+replace.
<<< END
