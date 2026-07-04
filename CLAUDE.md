# CLAUDE.md — oh-my-ib

**Read [AGENTS.md](AGENTS.md) first** — it is the canonical agent-conventions doc: project map, the
**agent-first** authoring rule, hard safety rules, and how this repo is built.

Critical backstops (full detail in AGENTS.md):

- **Public repo** — never commit account ids, tokens, or any secret.
- **Writes are gated** — Phase 2 (2026-07-03) added `buy`/`sell`/`cancel` (STK, LMT/MKT, DAY). Paper (`:4002`, the default) is ungated; **live orders require BOTH `--live` AND `OMI_ALLOW_LIVE=1`**. All other commands remain read-only; no modify, no combos. Options: data read + single-leg ORDERS (`option-buy`/`option-sell`, LMT/DAY), same gates. Write code lives ONLY in `src/ib/trade.rs`.
- **Pipeline-driven** — read `CONTRACT.md` in `jackypanster/pipeline`, then `.pipeline/<feature>/`; do not hand-edit out of band, run the stages.
