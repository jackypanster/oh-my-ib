# oh-my-ib

A Rust CLI (`omi`) that talks to **Interactive Brokers** via the TWS API
(`ibapi` crate, sync client) through a local **IB Gateway**. Designed to be driven
by an LLM agent over chat: the user gives natural-language instructions, the agent
runs `omi` subcommands, parses JSON output, and reports back.

**Phase 1 (current): read-only daily driver.** No order placement anywhere in the
codebase — structurally read-only. Trading is a later, separately-shipped phase.

**Hard safety rules (apply to every phase):**

- Paper account is the default (IB Gateway port `4002`). Live (`4001`) requires an
  explicit `--live` flag; any *write* additionally requires `OMI_ALLOW_LIVE=1`.
- This repo is **public**: never commit account ids, tokens, or any credential.
  Real config lives at `~/.config/oh-my-ib/config.toml` (outside the repo).
- TWS API authenticates via the logged-in IB Gateway — there are **no API keys**.

Approved design: see `.pipeline/phase1-readonly/PRD.md` (and `arch.md` once written).

---

> **This project is developed via the `pipeline` + `pipeline-dashboard` toolchain — a forge-agnostic,
> machine-agnostic, LLM-agnostic agent dev pipeline whose only durable asset is a git+markdown state
> bus under `.pipeline/`.**
>
> **How it works.** All work flows through staged commands `pipeline-prd → pipeline-arch →
> pipeline-task → pipeline-impl → pipeline-review`, plus `pipeline-hunt` for blocked cards. Each
> command is a ~20-line shim that does the same
> loop: `git pull --rebase` → read `.pipeline/current.json` + the feature's `journal.md` → resolve the
> stage's skill via `.pipeline/roles.yaml` → invoke that skill (it *reasons*; the shim owns all I/O) →
> write only its stage's write-set → append one entry to `.pipeline/<feature>/journal.md` → commit once
> → git push → print a self-contained handoff for the next (cold, possibly different-LLM) node. There is **no
> shared memory, no scheduler, no DB**: a human relays the printed handoff between bots, and any agent
> rebuilds full state from `git pull` alone.
>
> **The source of truth is `journal.md`** (append-only; its physically-last entry = the live position).
> `current.json` is only a fast cache — on disagreement the journal tail wins. The state machine is
> frozen: `todo → in-progress → review → done`, `blocked` terminal, `attempts ≥ 3 ⇒ blocked ⇒ hunt`.
> **Hard invariants you must never violate:** only `pipeline-review` merges, and only after explicit
> human confirmation; never edit a card's frozen `spec-paths` (the test gate — re-route to
> `pipeline-task` to re-freeze instead); never force-push trunk/shared refs; stay inside your stage's
> write-set; metadata lives on trunk, reviewed code on a `feat/<feature>` branch via PR.
>
> **To act:** read `CONTRACT.md` in [`jackypanster/pipeline`](https://github.com/jackypanster/pipeline)
> first (it is the single normative spec), then this repo's `.pipeline/<feature>/PRD.md` + `arch.md` +
> the journal tail. Do **not** hand-edit work out of band — run the stages.
>
> **To observe:** [`jackypanster/pipeline-dashboard`](https://github.com/jackypanster/pipeline-dashboard)
> is a read-only static-site generator. Run `node dist/cli.js /path/to/repo --out board.html` to render
> any `.pipeline/`-bearing checkout as a single `board.html` — feature stage flow, card lanes, and the
> run-journal timeline (who ran each stage, what transitioned, what failed, what's next), with a
> feature-level blocked banner. It never writes to the observed repo.
>
> **To auto-advance the `impl` loop (optional):** instead of hand-relaying each `impl` card, the
> repetitive `impl` multi-card loop can be run by
> [`jackypanster/pipeline-driver`](https://github.com/jackypanster/pipeline-driver) — a deterministic,
> **human-operated** loop (**an agent cannot run it unattended**: its GATE 1 blocks on a human reading the
> frozen red test and echoing its `spec-rev`) that runs `pipeline-impl` on a cheap model and **HALTS at
> every gate** (it never merges; the human runs `pipeline-review`). It is the human-operated write-side
> twin of the dashboard, scoped to `impl` ONLY. Every other stage stays human-relayed — **do not build any
> other scheduler**; the pipeline deliberately has none (see `DESIGN.md`).
