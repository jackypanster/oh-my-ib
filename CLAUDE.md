# CLAUDE.md — oh-my-ib

**Read [AGENTS.md](AGENTS.md) first** — it is the canonical agent-conventions doc: project map, the
**agent-first** authoring rule, hard safety rules, and how this repo is built.

Critical backstops (full detail in AGENTS.md):

- **Public repo** — never commit account ids, tokens, or any secret.
- **Writes are gated** — `buy`/`sell`/`cancel` (STK, LMT/MKT, DAY) plus `reprice` (same order id; `--confirm` to send). Paper (`:4002`) is ungated; **live orders require BOTH `--live` AND `OMI_ALLOW_LIVE=1`**. Options: data read + ORDERS (`option-buy`/`option-sell`/`option-combo`/`option-close`, LMT/DAY), same gates. Write code lives ONLY in `src/ib/trade.rs`.
- **Pipeline-driven** — read `CONTRACT.md` in `jackypanster/pipeline`, then `.pipeline/<feature>/`; do not hand-edit out of band, run the stages.
