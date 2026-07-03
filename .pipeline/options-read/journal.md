# journal — options-read (append-only)

## seq=1 · 2026-07-03T16:06:28Z · prd→arch · completed · by=claude-fable-5
done:   PRD decision-complete (operator /think-approved 2026-07-04): two READ-ONLY commands —
        option-chain (conid resolve + reqSecDefOptParams End-bounded drain, --exchange SMART
        default, sorted expirations/strikes) and option-quote (OptionBuilder + snapshot drain,
        greeks BEST-EFFORT: omit-if-absent, never an error). D1-D8 locked; trade.rs/write
        gates untouched; docs line "no options" → "no option ORDERS" rides the PR. Acceptance
        paper-first; Tiger reqSecDefOptParams support = journaled live observation.
output: .pipeline/options-read/PRD.md
--- handoff ---
>>> NEXT
Run pipeline-arch on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
Model: frontier SOTA required — operator assigns the bot; the pipeline can't verify the model.
First: git pull --rebase; no .env in this repo.
Read for context (before acting):
  - AGENTS.md + CLAUDE.md — repo conventions + the Phase-2 line you AMEND ("no options" → "no option ORDERS"; writes stay STK-only)
  - .pipeline/options-read/PRD.md — criteria 1-8, decisions D1-D8, non-scope
  - ~/.cargo/registry/src/*/ibapi-3.1.0/src/contracts/sync.rs (option_chain, ~line 267: Subscription<OptionChain>; End ⇒ Error::EndOfStream via contracts/common/stream_decoders.rs:50), contracts/builders.rs (OptionBuilder), market_data/realtime/mod.rs (~line 340: TickTypes::OptionComputation — iv/delta/gamma/vega/theta/underlying_price all Option<f64>)
  - src/ib/quote.rs — snapshot-drain house pattern (ADR 0013) that option-quote mirrors
  - src/ib/contract.rs — contract_details conid-resolve pattern that option-chain reuses
  - .pipeline/completed-orders/docs/adr/0015 + 0016 — the End-bounded drain class (chain drain's kin)
Your task (concrete, numbered):
  1. Pin exact call shapes from crate source: option_chain drain termination + OptionChain row mapping; OptionBuilder required fields and defaults (multiplier? exchange? currency?); OptionComputation variants — which TickType field values arrive in snapshot mode and which row(s) to emit as `greeks` (model vs bid/ask computation).
  2. Write arch.md: two new modules (src/ib/option_chain.rs, src/ib/option_quote.rs), pure seams (chain shaping incl. ascending sort; greeks extraction incl. omit-if-absent), exact CLI arg structs verbatim, mod.rs/main.rs wiring, and the AGENTS.md/CLAUDE.md amendment TEXT verbatim (so impl copies it).
  3. ADR 0019 (0017/0018 taken by stk-orders): options read-path bounded drains — End-bounded chain drain (decide explicitly whether it gets an ADR-0012-style timeout wrap; wedge dossier rule: every wait bounded) + SnapshotEnd quote drain + greeks-best-effort contract.
  4. CONTEXT.md — glossary: option chain, trading class (SPX vs SPXW), right, multiplier, greeks, OptionComputation, delayed model greeks, reqSecDefOptParams.
  5. Pin freeze coverage: frozen = pure seams + arg-validation matrix (right/strike/expiry) + --help + dead-port envelope; review-by-reading = gateway drain fns; live = criterion 8 paper acceptance.
Feature gotchas (project-specific traps the next node MUST know):
  - quote.rs output is FROZEN byte-identity (ADR 0013) — do NOT touch quote.rs even where copy-paste tempts; option-quote is a NEW module.
  - greeks may NEVER arrive under delayed+snapshot — omit keys, never error (PRD D3); do not make greeks presence a frozen assertion.
  - chain Subscription terminates via Error::EndOfStream mapped INTERNALLY by the crate — iterate like the completed-orders drain (ADR 0015/0016), do not hand-roll an End sentinel.
  - Tiger live (:4001) support for reqSecDefOptParams UNKNOWN — acceptance is paper (:4002); journal the live observation, never block on it.
  - This feature is READ-ONLY: no trade.rs edits, no write-gate changes; review polarity is the NORMAL read-only grep again (unlike stk-orders).
Done when: arch.md + CONTEXT.md + docs/adr/0019-*.md committed (+ journal seq=2 + current.json stage=arch) and PUSHED. On success: run pipeline-task.
On failure: attempts++; >=3 ⇒ blocked ⇒ run pipeline-hunt.
<<< END
