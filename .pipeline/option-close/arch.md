# Architecture — option-close

Two-card feature on the verified Phase-2 machinery. Card 01: read-side row identity
(pure-seam change in `positions.rs`, brief parity free). Card 02: `option-close` write verb
in `trade.rs` (the ONLY write module), reusing `build_option_order` + `place_with_client`
VERBATIM. Zero new safety machinery (ADR 0017/0018/0020/0021 apply as-is); one new ADR
(0022) for the close-by-conid semantics.

## Code-first verification results (grill findings — all checked against real code)

1. **Portfolio stream carries full identity**: `ibapi-3.1.0` `decode_contract`
   (`proto/decoders.rs:84`) fills `security_type`, `last_trade_date_or_contract_month`,
   `strike`, `right: Option<OptionRight>`, `multiplier: String`, `trading_class`,
   `currency` on every `AccountPortfolioValue.contract`. All struct fields are `pub` —
   synthetic values are constructible in integration tests.
2. **The 9-key row shape was NEVER frozen**: `position_row` is `pub(crate)` — unreachable
   from `tests/`. `brief_command.rs` passes prebuilt JSON rows into `assemble_brief`
   (passthrough assertions — unaffected by row-shape change). `claude_md.rs` asserts only
   pointer+`<900` bytes; `agents_md.rs` only substantive-size. **Consequence: NO existing
   suite re-freeze. Card 01's new spec is the FIRST freeze of the row shape** (refines PRD
   criterion 10, which over-estimated the ripple).
3. **`SecurityType` Display = IB wire code** (`"STK"`/`"OPT"`, contracts/mod.rs:96).
   Existing inconsistency observed: `contract.rs:38` emits Debug (`"Stock"`) — legacy,
   NOT changed here (out of scope; noted in CONTEXT.md).
4. **`multiplier` is a String in ibapi**; house style is string passthrough
   (`option_chain.rs:54`) or literal `"100"` (`option_quote.rs:87`).
5. **conid-addressed placement exists in the protocol** (`orders/common/verify.rs:207`,
   PLACE_ORDER_CONID) but is UNVERIFIED on the Tiger gateway ⇒ PRD D4 stands: rebuild via
   the live-proven builder chain + `contract_details` first-row assert (ADR 0021 pattern).
6. **CLAUDE.md budget**: 861 bytes today; amendment `+"/`option-close`"` = +15 ⇒ 876 < 900 ✓
   (frozen test `claude_md.rs` keeps passing; impl re-verifies byte count before PR).
7. **CLI house style for option verbs**: every field `#[arg(long)]`; `--limit` required
   `f64`; optional fields `Option<T>` (`OptionOrderArgs` precedent).

## Card 01 — positions identity enrichment (read-side)

**File**: `src/ib/positions.rs` only. **Seam promotion**: `position_row` `pub(crate)` → `pub`
(pure fn, read-only — the frozen spec constructs synthetic `AccountPortfolioValue`s and
asserts the exact JSON; `assemble_brief` pub-for-testability precedent).

Row: exact **14 keys** = existing 9 (order untouched) + 5 appended:

| key | value | non-OPT rows |
|---|---|---|
| `sec_type` | `contract.security_type.to_string()` (Display: `"STK"`, `"OPT"`, …) | always a string |
| `expiry` | raw passthrough `contract.last_trade_date_or_contract_month` | `null` |
| `strike` | `contract.strike` (number) | `null` |
| `right` | `Some(Call)⇒"C"`, `Some(Put)⇒"P"` (non_exhaustive-safe match) | `null` |
| `multiplier` | passthrough string; empty string ⇒ `null` | `null` |

**Null rule (locked)**: the 4 option keys are populated **iff `security_type == Option`**;
any other sec_type ⇒ all four `null` (deterministic, freeze-able; FUT/WAR expiry surfacing
is out of scope). `right` unmappable (non_exhaustive future variant) ⇒ `null`.
**Brief parity is automatic** — same fn; no brief.rs change.

## Card 02 — `option-close` (write verb)

**Files**: `src/cli.rs` (+`OptionCloseArgs`, `OptionClose` variant), `src/main.rs` (dispatch
arm), `src/ib/trade.rs` (ALL new logic), `src/ib/mod.rs` (re-export if house pattern needs),
AGENTS.md + CLAUDE.md amendment.

```rust
/// omi option-close --conid 123456789 --limit 3.20 [--qty 1]
pub struct OptionCloseArgs {
    #[arg(long)] pub conid: i32,     // from `omi positions`
    #[arg(long)] pub limit: f64,     // REQUIRED — LMT-only; finite > 0
    #[arg(long)] pub qty: Option<f64>, // whole >= 1; default = full position
}
```

### Data flow (single-connect — ONE client end to end)

```
option_close(cfg, args)
  1 usage    : conid >= 1; limit finite > 0; qty (if given) finite ∧ whole ∧ >= 1
  2 config   : require_live_write_gate(cfg)              [offline-deterministic]
  3 connect  : super::connect(cfg)                        → client
  4 account  : super::resolve_account(&client, cfg)
  5 match    : drain client.account_updates(&account) to End;
               rows with contract.contract_id == conid; LAST match wins (latest snapshot)
               ├─ no row, or row.position == 0  ⇒ not_found ("no open position for conid N
               │   — nothing to close; see `omi positions`")   [ANTI-OPEN GATE]
               └─ row.security_type != Option   ⇒ usage ("conid N is <SEC_TYPE>, not an
                   option — use `omi sell`/`omi buy` for stock")
  6 derive   : derive_close(position, qty) [pure FROZEN seam]
               side = position > 0 ⇒ SELL | position < 0 ⇒ BUY   [ANTI-DOUBLE GATE]
               close_qty = qty.unwrap_or(|position|); qty > |position| ⇒ Err(over-close)
  7 rebuild  : parse_expiry(raw row expiry) — unparseable ⇒ data error naming the raw value;
               (contract, order) = build_option_order(symbol, expiry, strike, right,
                 trading_class (row, non-empty only), "SMART", currency (row | "USD"),
                 side, close_qty, limit)                  [VERBATIM reuse, ADR 0020 D8]
  8 assert   : client.contract_details(&contract) FIRST row .contract_id == args.conid
               else data error ("resolved conid X != requested N — refusing to place")
                                                          [WRONG-CONTRACT GATE, ADR 0021]
  9 place    : place_with_client(&client, "option-close", &contract, &order, ack)
               [bounded first-ack, no-retry, timeout names order id — ADR 0017 VERBATIM]
```

### Pure FROZEN seams (card 02 spec surface)

- `derive_close(position: f64, qty: Option<f64>) -> Result<(Action, f64), String>` —
  Err on `position == 0`, on over-close, (qty pre-validated at step 1; seam re-checks
  whole ∧ >= 1 for totality). The SIGN of the held position is the ONLY side authority.
- `shape_option_close_ack(order_id, status, conid, symbol, expiry, strike, right, action,
  quantity, limit_price) -> Value` — exact **10 keys**, echoing the RESOLVED identity
  (from the matched row), `action` the DERIVED side, `expiry` the raw row string.
- Gateway fn `option_close` itself is review-by-reading (needs a live gateway), same as
  every sibling verb; its validation ORDERING (usage < config < connection) is frozen
  offline via the dead-port matrix (option-orders precedent).

### Error envelope map (frozen where offline)

| condition | code | offline-testable |
|---|---|---|
| conid < 1 / limit ≤ 0, ±inf, NaN / qty 0, fractional, inf | `usage` | yes |
| `--live` or effective `:4001` without `OMI_ALLOW_LIVE=1` | `config` | yes |
| paper dead port (validation+gate pass) | `connection` | yes |
| conid not held / position already 0 | `not_found` | no (gateway) |
| held but not OPT | `usage` | no (gateway) |
| expiry unparseable / conid-assert mismatch / stream errors | `data` | no (gateway) |
| no ack in window | `timeout` (exit 6, names order id) | no (gateway) |

## Docs amendment (two-text rule, option-orders lesson)

- **CLAUDE.md** (short form, 861B): options sentence gains `/`option-close`` ⇒ 876B < 900.
  Impl MUST `wc -c` before PR (frozen `claude_md.rs` is the backstop).
- **AGENTS.md** (full form): Phase-2 line gains `option-close` with the one-phrase
  semantics: close-by-conid, side derived from held position, LMT/DAY, same gates.

## Non-goals confirmed at arch (unchanged from PRD)

No MKT/GTC/modify; no auto-pricing; no combo whole-structure close; no exercise; no STK
close; no close-all; no new dependency; `positions()`/`brief()` gateway fns untouched
beyond the row seam.

## Freeze plan handed to task (advisory)

- Card 01 spec: `tests/positions_row.rs` (new) — synthetic STK row ⇒ 14 keys with 4 nulls +
  `sec_type:"STK"`; synthetic OPT row (incl. trading_class, multiplier "100", short & long
  qty) ⇒ populated; empty multiplier ⇒ null; non_exhaustive right fallback ⇒ null.
- Card 02 spec: `tests/option_close_command.rs` (new) — `derive_close` matrix (long⇒SELL,
  short⇒BUY, default full, partial, over-close Err, zero Err); 10-key ack exact; usage
  matrix (conid 0|-1, limit 0|inf|NaN|missing, qty 0|1.5|inf) pre-connect; gate matrix
  parity (no-env config / effective-port / paper-dead connection); `--help` lists the verb.
  Deliberate gate-pass omission (live-order hazard — option-orders/combo precedent).
- Card verify MUST be card-scoped (`cargo test --test <file>`); full-verify stays
  `[cargo build, cargo test]`.
