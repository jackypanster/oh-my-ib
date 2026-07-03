# arch — multi-quote

How the PRD's variadic quote lands. Binding decisions in **ADR 0013**
(`docs/adr/0013-batch-snapshots-one-session.md`); glossary deltas in `CONTEXT.md`. Every ibapi
claim verified against the vendored crate source (`~/.cargo/registry/src/*/ibapi-3.1.0/`).

## Design shape (three touched files, no new dependency)

| file | change |
|---|---|
| `src/cli.rs` | `QuoteArgs.symbol: String` → `symbols: Vec<String>` (positional, `#[arg(required = true, value_name = "SYMBOL")]`, help "Ticker symbol(s), e.g. AAPL MSFT"); other flags untouched (shared by the batch) |
| `src/ib/quote.rs` | extract per-symbol fetch seam `quote_one`; add pure N-shaping seam `shape_quotes`; `quote()` becomes guard → connect → switch once → ordered loop → shape |
| `src/ib/mod.rs` | re-export `shape_quotes` (frozen spec imports it) |

NOT touched: `src/main.rs` (`Command::Quote(args)` dispatch unchanged), `src/output.rs`
(generic renderer handles arrays), `src/error.rs`, every other `ib/` module, all existing tests.

## ibapi facts (source-verified, 2026-07-03)

- `market_data()` allocates a **fresh request-id per call** (`client.request()` builder,
  market_data/realtime/sync.rs:186-197) — the request-id routing domain; back-to-back snapshots
  on one session cannot cross-talk (same isolation ADR 0010 relies on).
- Subscription drop sends `CancelMarketData(request_id)` (market_data/realtime/mod.rs:379) —
  harmless after `SnapshotEnd` (line already released); the loop's consume-then-drop per symbol
  keeps at most ONE line open at a time (no pacing exposure).
- `switch_market_data_type` is a connection-level shared request
  (market_data/realtime/sync.rs:176-182) — called ONCE before the loop, applies to every
  subsequent snapshot on that client.
- Snapshot streams terminate with `TickTypes::SnapshotEnd` — bounded drain; explicitly NOT the
  markerless take-first class, so NO ADR 0012 timeout wrapping (PRD D4 records the distinction).

## Component design (impl follows this verbatim)

`src/ib/quote.rs`:

```rust
/// One symbol's snapshot on an already-connected client — returns EXACTLY today's
/// single-symbol object {symbol, delayed, ticks{…}}. Error contexts carry the symbol.
pub(crate) fn quote_one(
    client: &Client,
    symbol: &str,
    exchange: &str,
    currency: &str,
    delayed: bool,
) -> Result<Value, AppError> {
    let contract = Contract::stock(symbol).on_exchange(exchange).in_currency(currency).build();
    let subscription = client.market_data(&contract).snapshot().subscribe()
        .map_err(|e| AppError::data(format!("market_data failed: {e}"), format!("quote/{symbol}")))?;
    // …identical SnapshotEnd drain + quote_price_tick filtering as today…
    // stream-error arm context: format!("quote/{symbol}")
    Ok(json!({ "symbol": symbol, "delayed": delayed, "ticks": ticks }))
}

/// The pure, FROZEN N-shaping seam: 1 row ⇒ the bare object (byte-identity with today),
/// 2+ rows ⇒ the bare array in given order. Empty ⇒ json!([]) (defensive; unreachable via
/// clap `required = true`).
pub fn shape_quotes(mut rows: Vec<Value>) -> Value {
    if rows.len() == 1 { rows.pop().expect("len checked") } else { Value::Array(rows) }
}

pub fn quote(cfg: &Config, args: &QuoteArgs) -> Result<Value, AppError> {
    // STK guard unchanged (rejects before connecting)
    let client = super::connect(cfg)?;
    // md-type switch ONCE (identical match block as today; `delayed` computed once)
    let mut rows = Vec::with_capacity(args.symbols.len());
    for symbol in &args.symbols {
        rows.push(quote_one(&client, symbol, &args.exchange, &args.currency, delayed)?);
    }
    Ok(shape_quotes(rows))
}
```

Byte-identity argument (PRD criterion 1): `quote_one` emits the SAME `json!` literal as today's
`quote`; for N=1 `shape_quotes` returns that object bare; serde_json key order is
deterministic (BTreeMap). The only visible N=1 delta is error CONTEXT strings gaining the
symbol (`"quote"` → `"quote/AAPL"`) — messages/codes unchanged. Recorded in ADR 0013 as an
accepted, agent-visible-only-on-failure delta (context is diagnostic, not contract; the frozen
dead-port tests assert `code`, not `context`).

Duplicates pass through (PRD criterion 6): the loop consumes `args.symbols` as given.

## Freeze coverage (pinned for pipeline-task)

- **Frozen (`tests/multi_quote.rs`, offline-deterministic):**
  - `shape_quotes` pure seam: 1 ⇒ bare object (assert the exact object comes back untouched);
    3 ⇒ array, input order preserved; 0 ⇒ `[]`; rows pass through unmodified (no key changes).
  - CLI: `omi quote` with NO symbol ⇒ usage envelope + non-zero exit (clap required);
    `omi quote AAPL MSFT` dead-port ⇒ `code="connection"` envelope (variadic parse works);
    `quote --help` mentions plural symbols.
  - House-red: imports `oh_my_ib::ib::shape_quotes` — unresolved until impl lands.
- **Untouched frozen surfaces (must stay green):** `tests/quote_ticks.rs` (quote_price_tick),
  `tests/data_commands.rs` (quote help mentions md-type; single-symbol dead-port envelope).
- **Review-by-reading:** the gateway loop — one connect, one switch, per-symbol contexts,
  sequential consume-then-drop, `quote_one` literal identical to today's assembly.
- **Live (operator, PRD criterion 9, merge gate):** same-session `omi --live quote AAPL` +
  `omi --live quote AAPL MSFT NVDA`; single object matches batch row shape.

## Risks re-checked at arch level

- Back-to-back snapshots: request-id isolation + one-line-at-a-time (above) — live acceptance
  is the final proof on the Tiger gateway.
- `--format table` on an array: `output.rs` generic dotted-prefix renderer already handles
  arrays (`positions[0].symbol` style, used by brief) — no change, verified by reading.
- Rollback: additive CLI change + one module refactor; revert restores single-symbol parsing.
