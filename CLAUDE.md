# CLAUDE.md — oh-my-ib

**Read [AGENTS.md](AGENTS.md) first** — it is the canonical agent-conventions doc: project map, the
**agent-first** authoring rule, hard safety rules, and how this repo is built.

Critical backstops (full detail in AGENTS.md):

- **Public repo** — never commit account ids, tokens, or any secret.
- **Read-only** — no order-placement code; trading is a later, gated phase.
- **Live gate** — paper (`:4002`) is the default; live (`:4001`) needs `--live`; future writes need `OMI_ALLOW_LIVE=1`.
- **Pipeline-driven** — read `CONTRACT.md` in `jackypanster/pipeline`, then `.pipeline/<feature>/`; do not hand-edit out of band, run the stages.
