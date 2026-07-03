# CONTEXT — brief-command

New domain terms for this feature. Base glossary: `.pipeline/phase1-readonly/CONTEXT.md`; reuses
**take-first** / **unset sentinel** (pnl-command), **drain-to-End** / pure-seam join
(executions-command), **discovery** / **sweep** (pnl-by-position). Only the deltas live here.

## Brief domain

- **Brief** — the composite daily snapshot: ONE gateway connection, six sections + `account` +
  `as_of`, one JSON document. The agent's highest-frequency query (`omi brief`).
- **Section** — one top-level composite key holding a source command's payload verbatim:
  `account_summary`, `pnl`, `pnl_by_position`, `positions`, `orders`, `executions`.
- **Hoisting rule** (PRD criterion 2) — the shared `"account"` wrapper key appears once at top
  level; each section nests its source command's payload MINUS that wrapper. Row shapes are
  untouched (rows that carry a per-row `account` field, like `positions`/`orders` rows, keep it).
- **Consolidated drain** (ADR 0011) — brief's single `account_updates` pass feeding three
  consumers at once: summary fields, position rows, and the sweep's `(conid, symbol)` discovery
  list. Exists so the singleton `reqAccountUpdates` is subscribed exactly once per brief.
- **Sequential fetch discipline** (ADR 0010) — every subscription fully consumed and dropped
  before the next request starts; fixed fetch order (resolve → as_of → drain → pnl → sweep →
  orders → executions). No concurrent subscriptions inside brief.
- **Routing domain** — which channel ibapi's message bus dispatches an incoming message to
  (`transport/routing.rs`): request-id, order-id/exec-id, shared-by-message-type, or singleton
  shared channels. ADR 0010's safety table is stated in these terms.
- **`as_of`** — the gateway's `server_time()` (UTC by construction), rendered ISO-8601
  (`YYYY-MM-DDTHH:MM:SSZ`). Server truth, not the local clock; NOT the account_updates
  `UpdateTime` (time-of-day only).
- **`assemble_brief`** — the frozen pure seam: `(account, as_of, six sections) -> Value`,
  exact top-level key set, pass-through.

## Conventions (feature-specific)

- JSON contract: `{ account, as_of, account_summary{...}, pnl{...}, pnl_by_position[...],
  positions[...], orders[...], executions[...] }` — every section byte-shape-identical to its
  source command (criterion 2).
- **Fail-fast, no partial** (PRD D3): first failed fetch ⇒ `{"error":{...}}` on stderr, non-zero
  exit, nothing on stdout. On failure the agent degrades to the individual commands.
- Quiet/flat account ⇒ `[]` sections, exit 0. Gateway down ⇒ existing connection-error contract.
- `--format table` renders the composite via the existing generic dotted-prefix renderer
  (`output.rs:44-75`, explicitly not frozen) — e.g. `pnl.daily_pnl`, `positions[0].symbol`; no
  output.rs changes.
- Read-only; `--live` permitted (ADR 0005); no section include/exclude flags (PRD non-scope).
- **Merge gate (PRD criterion 10)**: operator live-accepts `omi --live brief`, cross-checked
  against the individual commands in the same session, BEFORE the PR merges.
