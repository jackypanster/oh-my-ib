# arch — snapshot-timeout

Inputs: `PRD.md` (D1–D7). Decision record: `docs/adr/0038-snapshot-total-deadline.md`. Glossary: `CONTEXT.md`.

## Shape

| File | Change |
|---|---|
| `src/ib/mod.rs` | + `pub const SNAPSHOT_DEADLINE: Duration = Duration::from_secs(20);` (doc: total per-snapshot deadline, > IB ≈11s tail, ADR 0038) · + `pub fn snapshot_timeout_error(instrument: &str, md_type: &str, context: &str) -> AppError` (pure) · re-export both (`pub use`/pub items reachable as `oh_my_ib::ib::{SNAPSHOT_DEADLINE, snapshot_timeout_error}`) |
| `src/ib/quote.rs` | `quote_one` gains `md_type: &str` param (label for the error; `quote()` derives it once from `cfg.md_type`) · bare `iter_data()` loop ⇒ total-deadline loop (ADR 0038 §Decision 2) · fix stale "Deliberately NOT wrapped…" comment (:40-45) to cite ADR 0038 |
| `src/ib/option_quote.rs` | same loop swap at :215 · instrument label `"{symbol} {expiry} {strike} {normalized_right}"` · fix stale module doc (:4-5) + inline comment (:212) to cite ADR 0038 |
| `src/error.rs` | `ErrorKind::Timeout` doc comment only (ADR 0038 §5); NO code change |

Untouched: `trade.rs`, all non-snapshot reads, CLI surface (no new flag/subcommand), success JSON.

## Loop (normative, both sites)

```rust
let deadline = Instant::now() + SNAPSHOT_DEADLINE;   // right after subscribe()
loop {
    let remaining = deadline.saturating_duration_since(Instant::now());
    match subscription.next_timeout(remaining) {
        Some(Ok(SubscriptionItem::Data(TickTypes::SnapshotEnd))) => break,
        Some(Ok(SubscriptionItem::Data(tick))) => { /* existing per-tick handling, unchanged */ }
        Some(Ok(SubscriptionItem::Notice(_))) => continue,           // iter_data parity
        Some(Err(e)) => return Err(AppError::data(format!("market_data stream: {e}"), ctx)),
        None if Instant::now() >= deadline => return Err(snapshot_timeout_error(label, md, ctx)),
        None => break,                                              // self-ended: old behavior
    }
}
```
`SubscriptionItem` import: `ibapi::prelude::SubscriptionItem` (📖 ibapi-3.1.0 prelude.rs:73; also
`ibapi::subscriptions::SubscriptionItem`, subscriptions/mod.rs:23). `remaining == 0` ⇒ `next_timeout` returns `None` immediately ⇒ classified timeout.

md-type label: `MdType::Live ⇒ "live"`, `Delayed ⇒ "delayed"`, `Frozen ⇒ "frozen"` (the CLI's own
spelling, `src/config.rs:26-28`); derived in the existing md-type `match` (quote.rs:32, option_quote.rs:195).

## Freeze coverage (for pipeline-task)

Frozen (offline, hermetic — `tests/snapshot_timeout.rs`, NEW spec file):
1. `SNAPSHOT_DEADLINE == Duration::from_secs(20)` and `> Duration::from_secs(11)` and `!= TAKE_FIRST_TIMEOUT`.
2. `snapshot_timeout_error("NVDA", "live", "quote/NVDA")`: `code()=="timeout"`, `exit_code()==6`,
   `context==Some("quote/NVDA")`, message contains `"NVDA"`, `"20s"`, `"md-type=live"`, `"--md-type delayed"`,
   and contains no `'\n'`.
3. Option label case: `snapshot_timeout_error("NVDA 20261002 225 C", "live", "option-quote")` ⇒ message
   contains the full label, context `option-quote`.
4. Regression: dead-port `quote AAPL` and `option-quote …` still `code="connection"` (pre-connect order
   unchanged); `quote --help` / `option-quote --help` gain no timeout flag.
Existing frozen specs (`multi_quote.rs`, `quote_ticks.rs`, `option_quote_command.rs`, `read_timeouts.rs`,
`data_commands.rs`) must stay green untouched.

NOT frozen (review-by-reading + operator live acceptance — needs a real gateway; no-mock rule):
the loop swap itself, Notice skipping, None classification, CancelMktData-on-drop, label wiring.
Live acceptance (weekend-reproducible on `:4001`): `omi --live --md-type live quote NVDA` ⇒ exit 6
within ≤ ~25s, stderr `code:"timeout"`, context `quote/NVDA`; `omi --live --md-type delayed quote SPY`
⇒ success with ticks (healthy path, ~12s).

## Reference-behavior table (external contract)

| Element relied on | Reference semantics | Our use | Tier |
|---|---|---|---|
| tickSnapshotEnd timing | "Expected to occur 11 seconds after beginning of request" (TWS API EWrapper ref + Streaming Data Snapshots) | deadline 20s > 11s | 📖 doc-cited |
| Silent snapshot (no tick/err/End) | undocumented | bounded by deadline | ✅ probed 2026-09-26 (NVDA, SPY, live md, market closed) |
| `Subscription::next_timeout` | `None` on window expiry OR end-of-stream; Data/Notice/Err via shared `handle_response` | Instant-classified None | 📖 ibapi-3.1.0 sync.rs:222-240,158-203 |
| Drop ⇒ cancel | `cancel()` skipped only if `snapshot_ended` | timed-out snapshot sends CancelMktData | 📖 sync.rs:78-99,284-289 |
| Notice range | `SubscriptionItem::Notice` = IB warnings 2100..=2169, non-terminal | skip | 📖 common.rs:18-24 |
| Healthy delayed tail | ≈12.7s wall incl. connect (observed) | must not timeout | ✅ probed 2026-09-26 |

Risk register (⚠️): none — every row is doc-cited or probed.

## Blindspot pass

### Highest-risk unknown unknowns
1. **Option snapshot tail with greeks may approach the deadline** — option computations (tick 13) can
   arrive late. Evidence: IB caps the snapshot at 11s regardless of tick type (doc-cited) ⇒ 20s holds.
   Cheap resolution: live acceptance of a healthy option-quote once OPRA is subscribed (operator item).
2. **`None` before deadline masks a dropped connection** — preserved old behavior (success with partial
   ticks). Not a regression; out of scope. Evidence: old `for` loop ended identically on `None`.
3. **Batch wall time** — N stuck symbols no longer multiply: fail-fast on the first (≤20s).

### Likely safe assumptions
- `Instant` is monotonic ⇒ no wall-clock skew issue.
- Error-code-bearing messages (354/10091/10168) still arrive as `Some(Err)` fast ⇒ `data` errors unchanged
  (probed today: 0.7–5.8s).
- `strike` f64 formats as `225` for 225.0 (Rust `{}` Display) ⇒ clean label; odd strikes (182.5) show `182.5`.

### Questions worth asking now
- None blocking. (Default md-type for `--live` is a Deferred PRD item.)

## PRD ⚠️ resolutions
- D2 (20s) ⇒ SETTLED 📖 doc-cited 11s tail + ✅ probed healthy 12.7s; `TAKE_FIRST_TIMEOUT` explicitly NOT reused.
- D4 (wording) ⇒ SETTLED: exact template pinned in ADR 0038 §Decision 3; frozen via substrings only.

## Card plan (hint for pipeline-task)
ONE card: `01-snapshot-deadline` — spec-paths `tests/snapshot_timeout.rs`; impl-paths `src/ib/mod.rs`,
`src/ib/quote.rs`, `src/ib/option_quote.rs`, `src/error.rs`. Stub-compile the spec against the pure
seams and run clippy on it before freezing (memory: frozen specs must be clippy-clean).
