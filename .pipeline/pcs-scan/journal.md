# Run journal — pcs-scan

## seq=1 · 2026-09-26T11:38:59Z · ∅→prd · completed · by=cc/claude-opus-5-5
done:   PRD for pcs-scan: read-only `omi pcs-scan SYM...` ranking one PCS candidate per symbol (1 expiry nearest 30 DTE
        in 21–45, short |Δ|≈0.20 from IB model greeks, long = short − $5, credit/max-loss rank, skip-not-fail).
        9 human-confirmed/code-verified + 3 ⚠️ assumed (D8 concurrent snapshots, D9 spot, D10 mid basis).
        control.json (coordinated, human-direct merge) on explicit operator request.
output: .pipeline/pcs-scan/PRD.md · .pipeline/pcs-scan/control.json
--- handoff ---
>>> NEXT
Run pipeline-arch on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
Model: frontier SOTA required (CC role). Mode: coordinated (control.json) — Pi=impl, grok=review+merge on human-direct token.
First: git pull --rebase; no .env (runtime config at ~/.config/oh-my-ib/config.toml — never commit it).
Read for context (before acting):
  - AGENTS.md (read FIRST) · .pipeline/pcs-scan/PRD.md (provenance-tagged decisions)
  - src/ib/quote.rs (quote_one ADR 0038 drain, quote_price_tick) · src/ib/option_quote.rs (option_quote_greeks, OptionBuilder)
  - src/ib/option_chain.rs (conid resolve + reqSecDefOptParams drain) · src/ib/mod.rs (SNAPSHOT_DEADLINE)
  - src/cli.rs · src/main.rs · src/surface.rs · tests/help_command.rs (inventory re-freeze)
  - .pipeline/snapshot-timeout/docs/adr/0038-snapshot-total-deadline.md
Your task (concrete, numbered):
  1. grill-with-docs the PRD; ⚠️ D8 (N concurrent Subscriptions on one blocking ibapi Client + batch cap), D9 (spot tick
     choice incl. Delayed* labels), D10 (mid basis, zero-bid legs) are MANDATORY challenge targets.
  2. Pin module layout + pure seam signatures (pick_expiry, select_pcs, pick_spot, rank, shape) for freezing.
  3. Emit arch.md + CONTEXT.md + ADR (next free number, skip-not-fail + batching).
Done when: arch.md + CONTEXT.md + ADR committed with journal seq=2, current.json.stage=arch.
<<< END

## seq=2 · 2026-09-26T11:41:35Z · prd→arch · completed · by=cc/claude-opus-5-5
done:   arch for pcs-scan: two-pass batched snapshots (coarse 8 strikes → interpolate k* → 3 nearest + −width partners),
        ≤50 concurrent subs/batch on one client (📖 Subscription owns Arc bus), skip-not-fail precedence, 11 pure frozen
        seams pinned. PRD ⚠️ D8/D9/D10 settled. ADR 0039.
output: .pipeline/pcs-scan/arch.md · CONTEXT.md · docs/adr/0039-pcs-scan-two-pass-batched-snapshots.md
--- handoff ---
>>> NEXT
Run pipeline-task on a FRESH session (assume you know nothing — rebuild from the repo + CONTRACT.md).
repo=git@github.com:jackypanster/oh-my-ib.git branch=main pr=none
Model: frontier SOTA required (CC role). Mode: coordinated (control.json) — Pi=impl, grok=review+merge on human-direct token.
First: git pull --rebase; no .env (runtime config at ~/.config/oh-my-ib/config.toml — never commit it).
Read for context (before acting):
  - AGENTS.md (read FIRST) · .pipeline/pcs-scan/{PRD.md,arch.md,CONTEXT.md,docs/adr/0039-*.md}
  - tests/snapshot_timeout.rs, tests/help_command.rs — frozen-spec style + the inventory to re-freeze
Your task (concrete, numbered):
  1. ONE card 01-pcs-scan: spec-paths tests/pcs_scan.rs + tests/help_command.rs (add "pcs-scan");
     impl-paths src/ib/pcs_scan.rs, src/ib/mod.rs, src/cli.rs, src/main.rs, src/surface.rs.
  2. Red spec per arch.md §Pure seams + §Freeze coverage (tie rules, bracket/no-bracket, Delayed* spot, zero-bid,
     reason strings, sort, every-symbol-once, usage errors, dead port, help flags/inventory/read-only).
  3. Verbatim stub-compile + clippy -D warnings on a scratch copy before the freeze commit.
Done when: freeze commit + card record commit pushed, journal seq=3, current.json.stage=task.
<<< END
