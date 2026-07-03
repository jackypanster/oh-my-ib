# arch — read-timeouts

How the PRD's bounded take-first reads land in this codebase. Binding decisions in
**ADR 0012** (`docs/adr/0012-take-first-timeout-twin.md`); glossary deltas in `CONTEXT.md`.
Every ibapi claim below is verified against the vendored crate source
(`~/.cargo/registry/src/*/ibapi-3.1.0/`), not guessed.

## Design shape (smallest correct diff)

Four touched files, no new dependency, no CLI/config surface, no output-shape change:

| file | change |
|---|---|
| `src/error.rs` | `ErrorKind::Timeout` variant → code `"timeout"`, exit `6`, constructor `AppError::timeout(message, context)` |
| `src/ib/mod.rs` | `pub const TAKE_FIRST_TIMEOUT: Duration = Duration::from_secs(10);` (pub — the frozen spec asserts value + location) |
| `src/ib/pnl.rs` | `pnl_with_client`: swap the blocking take-first read for the timeout twin; `None` arm → timeout error |
| `src/ib/pnl_by_position.rs` | `sweep_pnl_singles`: same swap; `None` arm → timeout error naming the conid |

NOT touched: `src/ib/brief.rs` (calls the two seams — inherits the bound for free),
`src/cli.rs`, `src/main.rs`, `src/output.rs` (`emit_error` renders via `code()`, generic),
every drain-to-End site (PRD non-scope). Exhaustiveness check (grep, 2026-07-03): `ErrorKind`
is matched ONLY inside `error.rs` (`code()`/`exit_code()`/`Display`); `next_data()` take-first
sites are exactly `pnl.rs:28` and `pnl_by_position.rs:76`.

## The mechanism (ADR 0012) — verified in ibapi-3.1.0 source

`next_data()` ≡ `iter_data().next()` (subscriptions/sync.rs:242-244). Its timeout twin ships in
the crate: `timeout_iter_data(Duration)` (sync.rs:279-281) — same notice filtering (`FilterData`),
each `.next()` blocks up to the timeout, yields `Option<Result<T, Error>>`:

- `Some(Ok(t))` — first data item (same as today).
- `Some(Err(e))` — stream error (same as today → `AppError::data`, unchanged).
- `None` — timeout expired **or** stream already ended (`stream_ended` short-circuits instantly,
  sync.rs:223-225) → `AppError::timeout` (the old "no PnL reading" arm collapses here; ADR 0012).

Rejected: calling `next_timeout(Duration)` directly — it yields `SubscriptionItem<T>` (Data OR
Notice, sync.rs:222), so the seams would re-implement notice filtering + deadline bookkeeping the
crate already provides. `timeout_iter_data(d).next()` is the exact data-only parallel of the
current `next_data()` call — a one-expression swap per seam.

## Exact seam diffs (impl follows this verbatim)

`src/ib/pnl.rs` (`pnl_with_client`):

```rust
// before
let reading = match subscription.next_data() {
    ...
    None => return Err(AppError::data("no PnL reading", ctx)),
};
// after
let reading = match subscription.timeout_iter_data(super::TAKE_FIRST_TIMEOUT).next() {
    Some(Ok(p)) => p,
    Some(Err(e)) => return Err(AppError::data(format!("pnl stream: {e}"), ctx)),
    None => return Err(AppError::timeout(
        format!(
            "no PnL reading within {}s — gateway PnL channel may be wedged; restart the gateway",
            super::TAKE_FIRST_TIMEOUT.as_secs()
        ),
        ctx,
    )),
};
```

`src/ib/pnl_by_position.rs` (`sweep_pnl_singles`): identical swap on `sub`; its `None` arm keeps
the sweep's fail-fast conid attribution:

```rust
None => return Err(AppError::timeout(
    format!(
        "pnl_single conid {conid}: no PnL reading within {}s — gateway PnL channel may be wedged; restart the gateway",
        super::TAKE_FIRST_TIMEOUT.as_secs()
    ),
    ctx,
)),
```

`Some(Ok)`/`Some(Err)` arms and everything around them stay byte-identical — PRD criterion 6
(healthy-path stdout unchanged on all four call paths) holds because only the no-data failure
path changes.

`src/error.rs`: add `Timeout` to the enum (doc: "a bounded read produced no data in time —
gateway-side wedge; the cure is a gateway restart"), `"timeout"` in `code()`, `6` in
`exit_code()` (1,2,3,4,5,64 taken — 6 free), constructor between `connection` and `not_found`
mirroring the siblings.

## Freeze coverage (pinned for pipeline-task)

- **Frozen (`tests/read_timeouts.rs`, offline-deterministic):**
  - `AppError::timeout(msg, ctx)` exists; `code() == "timeout"`; `exit_code() == 6`; `Display`
    renders `[timeout] msg (ctx)`.
  - Existing code/exit table unchanged (connection=2 · not_found=3 · data=4 · config=5 ·
    usage=64 · other=1) — regression-pins the envelope contract around the new variant.
  - `oh_my_ib::ib::TAKE_FIRST_TIMEOUT == Duration::from_secs(10)` (D3: fixed, shared, 10s).
  - Black-box: `omi --help` unchanged surface (no new subcommand/flag); `omi pnl` dead-port →
    still `code":"connection"` (the timeout must not shadow connect errors).
  - House-red: the file imports `AppError::timeout` + `TAKE_FIRST_TIMEOUT` — unresolved until
    impl lands (the `pnl_by_position_command.rs` freeze pattern).
- **Review-by-reading (NOT freezable offline — no fake IB server, no-mock rule
  `agent_docs/tests.md`):** the two seam swaps themselves (correct const, cure message with
  duration + conid attribution, `Some(Err)` arm untouched, no other reads modified).
- **Live (operator, PRD criterion 8):** `omi --live pnl` + `omi --live brief` healthy-path PASS
  in seconds (the timeout must not slow the happy path).

## Risks re-checked at arch level

- **Per-item window**: `timeout_iter_data` restarts the 10s window per yielded item, so filtered
  notices extend the total wait. Accepted (ADR 0012): each extension requires actual gateway
  traffic; the live-proven wedge is a SILENT channel ⇒ exits at 10s sharp. Worst case for
  `brief`: (1 + N positions) × 10s, serial (PRD criterion 3).
- **None-collapse**: a stream closed before data (previously `data`/"no PnL reading") now reports
  `timeout` — instantly, not after 10s (`stream_ended` short-circuit). Wording generalizes;
  exit-code class changes 4→6 for that (never-observed-live) corner. Accepted in PRD D1.
- brief's fetch order and subscription lifecycle (ADR 0010/0011) are untouched — the seams'
  signatures do not change, only their internal read call.
