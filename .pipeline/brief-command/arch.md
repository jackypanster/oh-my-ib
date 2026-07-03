# arch — brief-command

Chosen shape for `omi brief` (PRD D1–D6 locked). All claims verified against ibapi-3.1.0 crate
source and this repo; citations inline. Binding decisions: ADR 0010 (one-session sequential fetch)
+ ADR 0011 (consolidated drain + shared builders), on top of ADR 0007/0008/0009 prior art.

## Components (write-set for impl)

- **`src/ib/brief.rs` (NEW)** — two items:
  - `pub fn assemble_brief(account: &str, as_of: &str, account_summary: Value, pnl: Value,
    pnl_by_position: Value, positions: Value, orders: Value, executions: Value) -> Value` — the
    **pure seam** (frozen): emits exactly the top-level key set
    `{account, as_of, account_summary, pnl, pnl_by_position, positions, orders, executions}`;
    pass-through, no re-shaping, no key invention.
  - `pub fn brief(cfg: &Config) -> Result<Value, AppError>` — the gateway fn (NOT frozen),
    ADR 0010 fetch order:
    1. `super::connect(cfg)` → `super::resolve_account(&client, cfg)`.
    2. `client.server_time()` → format `as_of` ISO-8601 UTC via inherent accessors (ADR 0011 §3;
       UTC by construction, `decoders/mod.rs:60-64`).
    3. **Consolidated drain** (ADR 0011): `client.account_updates(&account)`, drain to
       `AccountUpdate::End` (`accounts/mod.rs:251-260`), simultaneously accumulating
       (a) summary fields via `account.rs`'s accumulator seam, (b) position rows via
       `positions.rs::position_row`, (c) the `(conid, symbol)` discovery list. Drop subscription.
    4. `pnl.rs` seam: take-first account PnL (ADR 0007) → 3-key object.
    5. `pnl_by_position.rs::sweep_pnl_singles(&client, &account, &discovery)` →
       `shape_pnl_by_position(rows)` (existing pure seam untouched).
    6. `orders.rs` seam: `all_open_orders` drain (`OpenOrderEnd` → EndOfStream,
       `stream_decoders.rs:70-72`) → array; `--account` filter semantics verbatim (ADR 0011 §Cons).
    7. `executions.rs` seam: `executions(filter)` drain (`ExecutionDataEnd`; commissions route
       ByExecutionId, `routing.rs:127-136`) → `merge_executions` array.
    8. `assemble_brief(...)`. **Fail-fast**: first error aborts (PRD D3), error names the failing
       section (e.g. `"brief/pnl"` context).
- **`src/ib/account.rs`** — extract the summary accumulator seam (absorb AccountValue
  `(key, value, currency)`; emit the 5-key object minus `account`; first-seen-currency-with-
  NetLiquidation behavior preserved, `account.rs:31-36`). `pub fn account(cfg)` keeps its own
  connect/drain/wrap — behavior unchanged.
- **`src/ib/positions.rs`** — extract `pub(crate) fn position_row(&AccountPortfolioValue) -> Value`
  (exact 9-key row incl. per-row `account`, `positions.rs:27-37`). `positions(cfg)` unchanged.
- **`src/ib/pnl.rs`** — extract a with-client take-first seam returning the 3-key object minus
  `account`. `pnl(cfg)` unchanged; `pnl_number` untouched.
- **`src/ib/pnl_by_position.rs`** — extract `pub(crate) fn sweep_pnl_singles(client, account,
  &[(i32, String)]) -> Result<Vec<PnlSingleRow>, AppError>` (the ADR 0009 sweep loop, discovery
  passed in). `pnl_by_position(cfg)` keeps its own discovery drain — unchanged.
- **`src/ib/orders.rs` / `src/ib/executions.rs`** — extract `*_with_client` seams returning the
  bare arrays. Public fns unchanged.
- **`src/ib/mod.rs`** — `mod brief;` + `pub use brief::{brief, assemble_brief};`.
- **`src/cli.rs`** — flat variant `Command::Brief`, doc
  `/// Daily account snapshot: summary, PnL, positions, orders, executions (one connection)`.
- **`src/main.rs`** — dispatch arm `Command::Brief => ib::brief(&config)`.
- **No changes** to `config.rs`, `output.rs`, `error.rs`, `tz.rs` — the generic dotted-prefix
  table renderer (`output.rs:44-75`) covers criterion 8 as-is.

## Interleaving safety (verified — ADR 0010 table)

Every dataset's routing domain verified in `transport/routing.rs` + module sources; strictly
sequential consume-then-drop discipline ⇒ no two live subscriptions, no cross-domain theft
(`CommissionsReport` routes ByExecutionId, never to the orders shared channel). The known Tiger
EAGAIN quirk is a reconnect artifact (`src/ib/mod.rs:38-48`) — inapplicable within one session.
Fallback deform recorded in ADR 0010 §4 (internal sequential sessions, distinct client_ids), only
on live-observed wedging, never preemptively.

## JSON contract (frozen keys)

```json
{"account":"<id>","as_of":"2026-07-03T02:06:15Z",
 "account_summary":{"net_liquidation":1.0,"total_cash":1.0,"buying_power":1.0,
                    "available_funds":1.0,"currency":"USD"},
 "pnl":{"daily_pnl":0.0,"unrealized_pnl":null,"realized_pnl":null},
 "pnl_by_position":[{"conid":1,"symbol":"AAPL","position":1.0,"daily_pnl":null,
                     "unrealized_pnl":null,"realized_pnl":null,"value":null}],
 "positions":[{"symbol":"AAPL","conid":1,"qty":1.0,"avg_cost":1.0,"market_price":1.0,
               "market_value":1.0,"unrealized_pnl":1.0,"realized_pnl":1.0,"account":"<id>"}],
 "orders":[{"order_id":1,"account":"<id>","symbol":"AAPL","conid":1,"action":"Buy",
            "quantity":1.0,"order_type":"LMT","limit_price":1.0,"aux_price":null,"tif":"Day"}],
 "executions":[{"exec_id":"x","order_id":1,"perm_id":1,"time":"t","symbol":"AAPL","conid":1,
                "side":"BOT","shares":1.0,"price":1.0,"cumulative_qty":1.0,"avg_price":1.0,
                "exchange":"SMART","commission":1.0,"commission_currency":"USD",
                "realized_pnl":null}]}
```

Sections byte-shape-identical to source commands (criterion 2); quiet account ⇒ `[]` arrays,
exit 0; fail-fast error envelope on stderr otherwise (criteria 5–7).

## Freeze coverage (for pipeline-task; per ADR 0006)

- **FROZEN** (`tests/brief_command.rs`, offline, card-scoped runner):
  - Black-box CLI: `omi --help` lists `brief`; `omi brief --help` exits 0; dead-port
    `omi brief` → non-zero exit + `{"error":{...}}` stderr (the `cli_contract.rs` pattern).
  - Pure seam via `use oh_my_ib::ib::assemble_brief`: exact top-level key set; `account`/`as_of`
    passed through; each section Value passed through unmodified (object and array cases);
    no extra keys.
- **NOT FROZEN** (reviewed-by-reading + operator live acceptance per PRD criterion 10): the
  `brief(cfg)` gateway fn — fetch order, consolidated drain, take-first/drain-to-End usage,
  as_of formatting, fail-fast mapping, section wiring; the six seam refactors' behavior
  preservation (sibling frozen tests pin `merge_executions`/`shape_pnl_by_position`/`pnl_number`
  + the CLI contract; `account`/`positions`/`orders` gateway paths are review-by-reading, as at
  their own birth).

## Risks carried forward (impl/review must know)

- **Merge gate (PRD criterion 10)**: operator live-accepts `omi --live brief` + same-session
  cross-check vs individual commands BEFORE merge. Gateway currently being (re)installed on the
  operator's new machine — the gate waits for the operator.
- The seam refactor touches five proven modules — impl must keep the public fns' behavior
  byte-identical (any drift in the six commands is a review REJECT even if brief itself is fine).
- Blocking reads: a wedged gateway hangs brief exactly like any sibling; `next_timeout` stays the
  recorded fallback (ADR 0007/0009), applied only on live-observed hangs.
- N-position sweep latency inside brief == pnl-by-position's accepted cost (D2 there); brief adds
  the other five fetches, still one connect.
