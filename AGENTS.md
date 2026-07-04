# AGENTS.md — oh-my-ib

Canonical agent-conventions for this repo. Read this first, then `CONTRACT.md` in
[`jackypanster/pipeline`](https://github.com/jackypanster/pipeline), then `.pipeline/<feature>/`.

## What this is
`omi` — a Rust CLI that reads an Interactive Brokers account over the TWS API (`ibapi`, sync client),
driven by an LLM agent: user gives natural-language instructions → agent runs `omi` subcommands →
parses JSON → reports. Reads everything; writes exist but are Phase-2 gated (see Hard safety rules). The operator runs a **Tiger Brokers
gateway** (TWS-API-compatible) live on `:4001`; drive it with `omi --live`. No API keys — auth is the
logged-in gateway. The gateway's `HKT` timezone is auto-registered, so no `IBAPI_TIMEZONE_ALIASES`
env var is needed.

## Authoring (agent-first)
**This repo is agent/LLM-facing. Write docs AND code for an agent to read and act on, not for human
readability.** When unsure "is this for a human or an agent?", assume agent.

- **Docs**: dense, structured, self-contained, machine-parseable. State facts, decisions, paths, and
  commands explicitly; no filler, no narrative-for-humans. The pipeline journal/handoff style is the model.
- **Code**: clear module boundaries; explicit types; stable machine-readable interfaces — JSON output
  with stable keys, deterministic exit codes; comments explain non-obvious behavior for the next agent.
- **Output**: agent-parseable first (JSON default); human/table views are secondary. Errors go to
  stderr as `{"error":{"code","message","context"}}` with a non-zero exit code.

## Hard safety rules
- **Writes are gated** — Phase 2 (2026-07-03) added `buy`/`sell`/`cancel` (STK, LMT/MKT, DAY). Paper (`:4002`, the default) is ungated; **live orders require BOTH `--live` AND `OMI_ALLOW_LIVE=1`**. All other commands remain read-only; no modify. Options: DATA readable (`option-chain`/`option-quote`); option ORDERS exist (`option-buy`/`option-sell` single-leg, `option-combo` multi-leg, `option-close` close-by-conid — side derived from held position; refuses while a working close order exists on the conid (anti double-fire); LMT/DAY) behind the same gates. Write code lives ONLY in `src/ib/trade.rs`.
- **Public repo**: never commit account ids, tokens, or any credential. Real config lives at
  `~/.config/oh-my-ib/config.toml` (outside the repo).
- **Live gate**: paper (`:4002`) is the default; live (`:4001`) requires an explicit `--live` flag;
  any future *write* additionally requires `OMI_ALLOW_LIVE=1`. The CLI refuses the live port without `--live`.

## How this repo is built
Pipeline-driven via [`jackypanster/pipeline`](https://github.com/jackypanster/pipeline): staged commands
`pipeline-prd → arch → task → impl → review` over a git+markdown state bus under `.pipeline/`.

- **Source of truth** = `.pipeline/<feature>/journal.md` tail (append-only; last entry = live position);
  `current.json` is a fast cache. To act: read `CONTRACT.md` (the single normative spec) first, then the
  feature's `PRD.md` + `arch.md` + journal tail. **Do not hand-edit work out of band — run the stages.**
- **Hard invariants**: only `pipeline-review` merges, after explicit human confirmation; never edit a
  card's frozen `spec-paths` (re-route to `pipeline-task` to re-freeze); never force-push trunk; stay in
  your stage's write-set; metadata on trunk, reviewed code on a `feat/<feature>` branch via PR.

## Verify
`cargo build` · `cargo clippy --all-targets -- -D warnings` · `cargo test`. Frozen specs live under
`tests/` (one `spec-rev` per feature); gateway-dependent behavior is reviewed-by-reading + operator
live acceptance (`omi --live <cmd>`).
