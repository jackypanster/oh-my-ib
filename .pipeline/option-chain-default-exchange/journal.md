# journal — option-chain-default-exchange

## seq=1 · 2026-07-06T05:13:46Z · prd→arch · completed · by=claude-opus-4-8
done:   PRD written. Fix the option-chain default-`--exchange` regression: on Tiger `:4001` the
        server-side `--exchange SMART` filter returns EMPTY (`{"chains":[]}`) though a SMART row
        exists unfiltered; `--exchange ""` returns 20 content-identical rows. Live-verified today
        (acct U20230856). HITL decision: move `--exchange` CLIENT-SIDE — query reqSecDefOptParams
        with `""` always, filter rows client-side; default `SMART` → 1 clean row, `""` → all,
        `<EX>` → that exchange. Read-only; `shape_option_chain` seam untouched; option-quote OUT.
output: .pipeline/option-chain-default-exchange/PRD.md
--- handoff ---
>>> NEXT
Run pipeline-arch on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
Model: frontier SOTA required — operator assigns the bot; the pipeline can't verify the model.
First: git pull --rebase; load repo config (.env if present, per CONTRACT step 2).
Read for context (before acting):
  - oh-my-ib/AGENTS.md + CLAUDE.md — repo conventions (agent-first authoring; write code lives ONLY in
    src/ib/trade.rs — N/A here, this is a READ path; hard safety rules)
  - .pipeline/option-chain-default-exchange/PRD.md — what + the HITL client-side-filter decision
  - src/ib/option_chain.rs — the target gateway fn + the pure shape seam (do NOT touch the shape seam)
  - src/cli.rs OptionChainArgs — the `--exchange` default (stays literal "SMART"; meaning changes)
  - tests/option_chain_command.rs — the FROZEN shape/CLI spec (stays green; the gateway fn's old
    "server-side passthrough" was NOT frozen, so changing it is in-bounds)
  - .pipeline/options-read/docs/adr/0019-* — the drain/timeout posture to reuse verbatim
Your task (concrete, numbered):
  1. Decide the client-side filter mechanism + emit an ADR. Recommended: a NEW pure, frozen-testable
     seam `filter_chain_rows(rows, &exchange) -> rows` ("" ⇒ passthrough; else retain row.exchange==EX,
     exact-string case-sensitive), applied in `option_chain` between the drain and `shape_option_chain`.
     Confirm: pass "" ALWAYS to `client.option_chain` (Tiger's server filter is unreliable).
  2. Decide filter-vs-sort order (observable result must be identical — criterion 7) and whether the
     filter seam sorts or shape_option_chain's existing sort suffices.
  3. Nail the edge: `--exchange SMART` on a gateway with no SMART row ⇒ `chains: []` (honest empty,
     criterion 4). And confirm option-quote is OUT (its SMART is a routing exchange, not a filter).
  4. Write arch.md + CONTEXT.md (glossary: server-side vs client-side exchange filter; SMART row) +
     docs/adr/NNNN-option-chain-client-side-exchange-filter.md. Do NOT write src or tests.
Feature gotchas (project-specific traps the next node MUST know):
  - The pure `shape_option_chain` seam is FROZEN (tests/option_chain_command.rs). The new filter must be
    a SEPARATE seam/step — never a shape edit.
  - reqSecDefOptParams on Tiger: server-side exchange filter DROPS SMART; a SMART row appears only in the
    unfiltered ("") result. Client-side exact-string `== "SMART"` yields the clean default.
  - `--exchange` default in cli.rs stays "SMART"; only its semantics change (server passthrough →
    client filter). Update the module doc comment (option_chain.rs:65) + fn doc.
  - This is a READ path — none of trade.rs / the write gate is involved.
Done when: arch.md + CONTEXT.md + the ADR exist; the filter seam signature is decided; current.json
stage=arch; journal seq=2 appended + pushed. On success: transition arch→task, run pipeline-task.
On failure: attempts++; >=3 ⇒ blocked ⇒ run pipeline-hunt.
<<< END
