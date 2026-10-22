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
- **Writes are gated** — Phase 2 (2026-07-03) added `buy`/`sell`/`cancel` (STK, LMT/MKT, DAY). Paper (`:4002`, the default) is ungated; **live orders require BOTH `--live` AND `OMI_ALLOW_LIVE=1`**. `reprice --price P ORDER_ID` modifies an open single-leg LMT (STK/OPT) by resubmitting `placeOrder` with the **same order id** (TWS modify; never cancel+replace). Without `--confirm` it prints a JSON preview (`transmits: false`) and sends nothing; with `--confirm` it submits and re-checks `OMI_MAX_NOTIONAL` against the new price (live only). The global `--preview` flag forces that preview (no `place_order`, live-write gate skipped) even if `--confirm` is set. Tiger's gateway has not been observed accepting this modify — paper acceptance is required before relying on it (ADR 0040). Options: DATA readable (`option-chain`/`option-quote`); option ORDERS exist (`option-buy`/`option-sell` single-leg, `option-combo` multi-leg, `option-close` close-by-conid — side derived from held position; refuses while a working close order exists on the conid (anti double-fire); LMT/DAY) behind the same gates. Orders are stamped with the resolved account (`--account` honored on writes). Write code lives ONLY in `src/ib/trade.rs`.
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

## Exit codes (write contract)

Aligned with the sister `ftnn` contract. JSON `code` and the process exit are both stable. Success (exit 0, ack JSON) is unchanged.

| exit | `code` | when |
|---|---|---|
| 0 | — | success. A write ack (including `status` `Inactive`/`Cancelled`) is still success — read `status`. |
| 2 | `connection` | gateway unreachable. |
| 3 | `not_found` | a requested entity does not exist (reads, option-close). |
| 4 | `data` | data or precondition error. On a write, the order was **NOT sent** (not open, not LMT, not STK/OPT, local `place_order` validation). |
| 5 | `config` | bad local config, flag combination, or the live-port env gate (`OMI_ALLOW_LIVE` missing). Frozen; not renumbered. |
| 6 | `timeout` | a bounded **read** produced no data. Not a write-submit timeout. |
| 7 | `rejected` | the broker rejected a submitted order (TWS notice codes 200–399, except cancel-confirmation 202). |
| 8 | `unconfirmed` | a write was sent but the outcome is unknown (ack timeout, dropped connection). **Callers must never auto-retry.** Verify with `omi orders`. |
| 64 | `usage` | bad argv / local validation. |
| 64 | `guard` | risk-cap or live posture block before send (`OMI_MAX_NOTIONAL` exceeded or invalid, live MKT, combo defined-risk refuse). Order NOT sent. Paper is exempt from the notional cap. |
| 1 | `error` | anything else. |

`buy`/`sell`/`cancel`/option writes use the same 4 / 7 / 8 / 64 buckets. Read timeouts stay exit 6.

## Verify
`cargo build` · `cargo clippy --all-targets -- -D warnings` · `cargo test`. Frozen specs live under
`tests/` (one `spec-rev` per feature); gateway-dependent behavior is reviewed-by-reading + operator
live acceptance (`omi --live <cmd>`).
