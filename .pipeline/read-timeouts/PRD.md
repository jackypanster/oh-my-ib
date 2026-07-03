# PRD — read-timeouts

Feature: bounded take-first reads — apply ADR 0007's recorded `next_timeout` fallback to BOTH
take-first read seams (`reqPnL`, `reqPnLSingle`), so no `omi` invocation can block forever on a
wedged gateway PnL channel.
Status: decision-complete (grilled 2026-07-03, operator locked D2/D3 below; mechanism and scope
verified against this repo's source and ibapi-3.1.0 crate source, not guessed).

## Problem

ADR 0007's hang trigger has FIRED in live acceptance (2026-07-03, gateway build 2026-06-25 on the
operator's M1 install — evidence: `.pipeline/brief-command/reviews/review-01.md` §Live acceptance):

- `reqPnL` / `reqPnLSingle` are **markerless** streams read via blocking `next_data()` with no
  timeout (take-first, ADR 0007/0009). On a wedged gateway PnL channel the first tick never
  arrives ⇒ `omi --live pnl` **blocked forever** (reproduced twice, incl. a fresh `--client-id`,
  before any process kill).
- A killed wedged process never sends the cancel ⇒ **orphaned reqPnL subscriptions pollute the
  channel for ALL clients** — a subsequent `omi --live brief` wedged at its pnl step too. Only a
  gateway restart cures it.
- ADR 0007 explicitly recorded `next_timeout(Duration)` as the fallback "if live acceptance
  reveals a hang" — that condition is now live-proven; this feature applies the recorded fallback.

Four call paths share the exposure through two seams: `omi pnl` + `brief/pnl` (via
`pnl_with_client`, `src/ib/pnl.rs`), and `omi pnl-by-position` + `brief/pnl_by_position` (via
`sweep_pnl_singles`, `src/ib/pnl_by_position.rs`).

## Goal

Every take-first read completes in bounded time. On timeout the command exits promptly with a
distinct, actionable structured error that names the cure (restart the gateway), so a consuming
agent can machine-distinguish "gateway wedged — restart it" from ordinary data errors.

## Success criteria (acceptance)

1. **Bounded `omi pnl`**: against a gateway session whose PnL channel emits nothing, `omi pnl`
   exits in ~10s (never blocks indefinitely) with `{"error":{"code":"timeout",…}}` on stderr,
   nothing on stdout, exit code 6.
2. **Bounded sweep**: `omi pnl-by-position` — any single `pnl_single` read stalling ⇒ same bounded
   exit; the error message names the offending conid.
3. **Bounded brief**: the same stall inside `omi brief` fail-fasts the whole command (brief D3,
   unchanged) with the timeout envelope; worst case bounded by (1 + N positions) × 10s.
4. Timeout message carries the wait and the cure, e.g.
   `no PnL reading within 10s — gateway PnL channel may be wedged; restart the gateway`.
5. `code":"timeout"` ⇔ exit code 6, new and unique (existing: connection=2, not_found=3, data=4,
   config=5, usage=64, error=1 — 6 is free).
6. **Healthy-path byte-identity**: on a normally-emitting gateway, all four call paths' stdout is
   byte-identical to today; the existing frozen suite stays green UNTOUCHED (this feature changes
   only the failure path).
7. `cargo build` · `cargo clippy --all-targets -- -D warnings` · `cargo test` green.
8. **Merge gate (operator, live):** `omi --live pnl` and `omi --live brief` PASS on a healthy
   gateway (~seconds, not 10s — the timeout must not slow the happy path). If the wedge happens to
   reproduce during acceptance, the bounded timeout exit is observed as bonus proof — NOT a
   required trigger (the wedge is not reproducible on demand).

## Scope

- `src/error.rs`: new `ErrorKind::Timeout` → code `"timeout"`, exit 6, `AppError::timeout(…)`
  constructor.
- ONE shared timeout constant (10s; exact home is arch's to pin) used by both seams.
- `pnl_with_client` and `sweep_pnl_singles`: replace the blocking take-first read with its timeout
  twin (D1); update the no-reading error arm to the timeout error + cure message.
- Nothing else — no new dependency, no CLI/config surface, no output-shape change.

## Non-scope (explicitly NOT this feature)

- **Drain-to-End reads** (`account_updates`, `open_orders`, `executions`) — they have End markers
  and a drain-side wedge has never been observed live; bounding them is a different failure class
  and a future feature if evidence appears.
- `quote` (bounded by SnapshotEnd), `history`/`contract`/`server_time` (request-response), the
  connect/handshake phase (own posture, `connect-retry` feature).
- No retry / auto-restart / channel-health probe; the cure stays operator-run gateway restart.
- No configurability of the timeout (D3) — no config key, no flag.
- No `--watch`, no async-client migration.

## Resolved decisions (locked)

- D1 **Mechanism = the timeout twin of `next_data()`** (code, ibapi-3.1.0 source):
  `next_data()` ≡ `iter_data().next()` (subscriptions/sync.rs:242-244); the crate ships
  `timeout_iter_data(Duration)` (sync.rs:279-281) — identical notice-filtering, each `.next()`
  waits up to the timeout, returns `None` on expiry. So the seams swap
  `subscription.next_data()` → `subscription.timeout_iter_data(TIMEOUT).next()` and map `None` to
  the timeout error. Consequence (accepted): the old `None` = "stream closed before data" arm
  collapses into the timeout error — a closed stream returns `None` immediately (sync.rs:223-225),
  so the exit is instant, only the wording generalizes. `Some(Err(…))` keeps its existing
  stream-error mapping.
- D2 **New `timeout` error code, exit 6** (operator, grilled 2026-07-03). Chosen over reusing
  `data`: an agent must machine-distinguish "restart the gateway" from gateway data errors without
  message sniffing. Envelope shape unchanged (`{"error":{code,message,context}}`).
- D3 **Fixed 10s shared const, not configurable** (operator, grilled 2026-07-03). Healthy first
  tick arrives <1s (live-observed); the wedge emits nothing forever, so tunability buys nothing.
  Zero config/flag surface.
- D4 **Scope = the two shared seams only** (review-01 routing recommendation; operator confirmed
  by starting this feature). Fixing `pnl_with_client` + `sweep_pnl_singles` covers all four call
  paths at once — the seam extraction done for brief (ADR 0010/0011) is what makes this a
  two-site diff.
- D5 **brief stays whole-command fail-fast** (inherited, brief PRD D3): a timeout in any brief
  step propagates as the command's single structured error; no per-section degrade.

## Risks / fragile assumptions

- **The timeout path is not black-box testable offline**: triggering it needs a server that
  completes the IB handshake then goes silent — a fake IB server, which the repo's no-mock rule
  (agent_docs/tests.md) forbids. Freezable surface is therefore the error-envelope mapping
  (`timeout` ⇔ 6) and the unchanged-CLI contract; the seam wiring itself is review-by-reading +
  live acceptance (criterion 8). `pipeline-task` must record exactly this split in the card's
  `## Freeze coverage`.
- `timeout_iter_data` restarts the 10s window per yielded item, so a stream emitting only filtered
  Notices could extend total wait beyond 10s. Accepted: notices are rare, each extension requires
  actual traffic — a silent wedge (the live-proven class) exits at 10s sharp.
- The wedge itself is a gateway-state hazard this feature does NOT cure — it bounds the damage
  (seconds, not forever; no more killed-process subscription litter) and makes the cure explicit
  in the error message.
- Rollback: two-seam failure-path diff + one error variant; revert restores blocking reads.

## Verification

- Offline: frozen spec (task freezes; card-scoped runner per CONTRACT §Multi-card) — the
  `timeout` code/exit mapping and the untouched sibling CLI contract; plus the full existing suite
  green (criterion 6/7).
- Live (operator): criterion 8 — healthy-path speed unchanged; timeout path proven by reading +
  (opportunistically) the next real wedge.
