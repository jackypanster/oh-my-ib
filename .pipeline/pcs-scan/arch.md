# arch — pcs-scan

Inputs: `PRD.md` (D1–D11). Decision record: `docs/adr/0039-pcs-scan-two-pass-batched-snapshots.md`. Glossary: `CONTEXT.md`.

## Shape

| File | Change |
|---|---|
| `src/ib/pcs_scan.rs` | NEW — gateway flow (`pcs_scan(cfg, args)`) + `snapshot_batch` helper + PURE seams below |
| `src/ib/mod.rs` | `mod pcs_scan;` + `pub use pcs_scan::{…pure seams, types, PCS_BATCH_LINES, pcs_scan}` |
| `src/cli.rs` | `Command::PcsScan(PcsScanArgs)`; `PcsScanArgs` (below); doc line `/// Rank put-credit-spread candidates over symbols (read-only)` |
| `src/main.rs` | dispatch `Command::PcsScan(args) => ib::pcs_scan(&config, args)` |
| `src/surface.rs` | `command_name` arm `"pcs-scan"` + registry entry (gate `read-only`, non-empty purpose/usage/example) |
| `tests/help_command.rs` | inventory set gains `"pcs-scan"` — RE-FREEZE by pipeline-task (spec-path of this feature) |

Untouched: `trade.rs`, all existing commands' behavior/output.

## CLI

```rust
pub struct PcsScanArgs {
    /// Underlying symbols (1+)
    #[arg(required = true)] pub symbols: Vec<String>,
    #[arg(long, default_value_t = 21)] pub dte_min: i64,
    #[arg(long, default_value_t = 45)] pub dte_max: i64,
    #[arg(long, default_value_t = 30)] pub dte_target: i64,
    /// Target |delta| of the short put
    #[arg(long, default_value_t = 0.20)] pub delta: f64,
    /// Spread width in dollars (long strike = short strike − width)
    #[arg(long, default_value_t = 5.0)] pub width: f64,
    /// Minimum credit_mid as a fraction of width
    #[arg(long, default_value_t = 0.25)] pub min_credit_ratio: f64,
}
```
Pre-connect validation (usage, exit 64, context `pcs-scan`) via pure `validate_params(&PcsParams) -> Result<(), String>`:
`dte_min < 0` or `dte_min > dte_max` or `dte_target ∉ [dte_min, dte_max]`; `delta` not finite or ∉ (0,1);
`width` not finite or ≤ 0; `min_credit_ratio` not finite or ∉ [0,1). Symbols upper-cased? NO — passed verbatim
(consistent with `quote`).

## Pure seams (FROZEN — `tests/pcs_scan.rs`), all in `src/ib/pcs_scan.rs`, re-exported at `oh_my_ib::ib::*`

```rust
pub const PCS_BATCH_LINES: usize = 50;

#[derive(Debug, Clone, PartialEq)]
pub struct PcsParams { pub dte_min: i64, pub dte_max: i64, pub dte_target: i64,
                       pub delta: f64, pub width: f64, pub min_credit_ratio: f64 }
impl Default for PcsParams { /* 21, 45, 30, 0.20, 5.0, 0.25 */ }

#[derive(Debug, Clone, PartialEq, Default)]
pub struct PutRow { pub strike: f64, pub bid: Option<f64>, pub ask: Option<f64>,
                    pub delta: Option<f64>, pub iv: Option<f64> }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason { NoExpiryInWindow, SpotUnavailable, NoGreeks, NoLongLeg, BelowMinCredit, QuoteError }
impl SkipReason { pub fn as_str(&self) -> &'static str }   // snake_case, PRD criterion 4

pub fn validate_params(p: &PcsParams) -> Result<(), String>;

/// Last>0 → mid(Bid,Ask) if both>0 → Close>0 → None. Keys are `quote_price_tick` labels; a `Delayed`
/// prefix is stripped before matching (DelayedLast ≡ Last …).
pub fn pick_spot(ticks: &serde_json::Map<String, serde_json::Value>) -> Option<f64>;

/// YYYYMMDD strings; malformed ignored. Among expiries with dte ∈ [dte_min, dte_max] (dte = days from
/// `today`), return (expiry, dte) nearest dte_target; tie ⇒ earlier. None ⇒ NoExpiryInWindow.
pub fn pick_expiry(expirations: &[String], today: time::Date, p: &PcsParams) -> Option<(String, i64)>;

/// Listed strikes in [0.65×spot, spot], ascending, dedup; ≤ n evenly spread by index
/// (idx = round(i×(len−1)/(n−1)), i=0..n) — both ends kept; len ≤ n ⇒ all.
pub fn coarse_strikes(strikes: &[f64], spot: f64, n: usize) -> Vec<f64>;

/// Rows with Some(delta), by strike asc. Find adjacent rows a<b (strike) with |Δa| ≤ target ≤ |Δb|;
/// k* = a.strike + (target−|Δa|)×(b.strike−a.strike)/(|Δb|−|Δa|) (equal |Δ| ⇒ a.strike).
/// No bracket ⇒ strike of the row whose |Δ| is nearest target. No delta rows ⇒ None.
pub fn estimate_target_strike(rows: &[PutRow], target: f64) -> Option<f64>;

/// The `n` listed strikes nearest k* (tie ⇒ lower), plus each one's (strike − width) when listed
/// (float-equal within 1e-6), ascending, dedup, excluding any strike in `already`.
pub fn refine_strikes(strikes: &[f64], k_star: f64, width: f64, n: usize, already: &[f64]) -> Vec<f64>;

pub struct PcsCandidate { symbol, spot, expiry, dte, short: PutRow, long: PutRow, width, credit_mid,
    credit_natural, max_loss, return_on_risk, breakeven, pop_approx, spread_pct }  // all pub, f64/String/i64

/// usable = bid>0 && ask>0. short = usable row with Some(delta) whose |delta| nearest p.delta (tie ⇒ lower
/// strike); none ⇒ NoGreeks. long = usable row with strike == short.strike − p.width (1e-6); none ⇒ NoLongLeg.
/// credit_mid < p.min_credit_ratio × p.width ⇒ BelowMinCredit. Math = PRD criterion 3
/// (mid=(bid+ask)/2; spread_pct = mean over legs of (ask−bid)/mid).
pub fn select_pcs(symbol: &str, spot: f64, expiry: &str, dte: i64, rows: &[PutRow], p: &PcsParams)
    -> Result<PcsCandidate, SkipReason>;

/// {"params":{dte_min,dte_max,dte_target,delta,width,min_credit_ratio},
///  "candidates":[…sorted by return_on_risk desc, tie ⇒ symbol asc…],
///  "skipped":[{"symbol","reason"}…in input order]}. Candidate JSON keys = PRD criterion 3
///  (short/long objects: strike,bid,ask,mid + short.delta, short.iv (null if None)).
pub fn shape_pcs_scan(p: &PcsParams, candidates: Vec<PcsCandidate>, skipped: Vec<(String, SkipReason)>) -> serde_json::Value;
```

## Gateway flow (`pcs_scan`, NOT frozen — review-by-reading + live acceptance)

1. `validate_params` (usage) → `super::connect(cfg)` → md-type switch (quote.rs pattern).
2. `today = time::OffsetDateTime::now_utc().date()` (UTC date; ≤1-day DTE skew accepted).
3. Spot pass: `Contract::stock(sym)` SMART/USD for all symbols via `snapshot_batch` → `pick_spot`.
4. Chain step per symbol with spot: `contract_details` FIRST row conid → `client.option_chain(sym,"",Stock,conid)`
   drained like `option_chain.rs` (timeout-wrapped) → row = SMART & trading_class==sym, else SMART with most
   expirations, else none (⇒ `quote_error`) → `pick_expiry`.
5. Pass 1 for all live symbols in one batched run: `coarse_strikes(row.strikes, spot, 8)` → `Contract::put(sym)
   .strike(k).expires_on(y,m,d).on_exchange("SMART").in_currency("USD").trading_class(row.trading_class)`.
   Per contract collect ticks (`quote_price_tick`: Bid/Ask incl. Delayed*) + last model greeks
   (`option_quote_greeks` → delta, implied_volatility) into `PutRow`.
6. Pass 2: `estimate_target_strike` → `refine_strikes(row.strikes, k*, width, 3, pass1_strikes)` → batched.
7. `select_pcs` over pass1 ∪ pass2 → candidate or skip; precedence per ADR 0039 §3; `shape_pcs_scan`.
`snapshot_batch`: chunks of `PCS_BATCH_LINES`; subscribe all in chunk (subscribe error ⇒ that contract Err),
shared deadline `Instant::now() + SNAPSHOT_DEADLINE`, drain each with the ADR 0038 loop (Notice skip, SnapshotEnd
end, Some(Err)/deadline ⇒ Err for that contract only), drop chunk before next.

## Freeze coverage (for pipeline-task)

Frozen: every pure seam above (incl. tie rules, bracket/no-bracket, Delayed* spot, zero-bid exclusion, skip reason
strings, sort order, every-symbol-once shape), `PCS_BATCH_LINES == 50`, CLI usage errors pre-connect (exit 64 /
code `usage`), dead port ⇒ `connection`, `pcs-scan --help` shows the 6 flags, `omi help` inventory + `read-only`.
Review MUST read: gateway flow 1–7, `snapshot_batch` concurrency/drop, chain-row choice, skip precedence wiring.
Live acceptance (after OPRA, RTH): PRD criterion 7.

## Reference-behavior table

| Element | Reference semantics | Use | Tier |
|---|---|---|---|
| concurrent snapshot subs, one client | `Subscription` owns Arc bus + request_id, no client borrow | batch ≤50 | 📖 ibapi sync.rs:38-49, realtime/sync.rs:186-196 |
| IB market-data lines | default 100 simultaneous | cap 50 | 📖 IBKR pricing/docs (default 100 lines) |
| snapshot tail | ≈11s | shared batch deadline 20s | 📖 ADR 0038 |
| model greeks tick | ModelOption(13)/DelayedModelOption(83) carry delta/IV | short-leg delta | 📖 ADR 0019 D3 |
| chain strikes | union across expiries | nonexistent strike ⇒ per-contract error ⇒ skip | ✅ probed 2026-09-26 (scan) |
| option quotes need OPRA | 354 without OPRA | live acceptance gated on OPRA | ✅ probed 2026-09-26 |

Risk register (⚠️): greeks presence under the operator's entitlement (R1) — verified only at live acceptance.

## Blindspot pass

### Highest-risk unknown unknowns
1. **Pacing on subscribe bursts** — IB limits ~50 msgs/s; a 50-contract burst is at the edge. Cheap fix if seen
   (error 100 "Max rate"): impl may insert a ≤20ms sleep between subscribes (reviewer accepts).
2. **Model greeks may arrive after bid/ask but before SnapshotEnd** — drain to SnapshotEnd keeps last model row
   (ADR 0019 D3 last-write-wins) ⇒ covered.
3. **Weekly vs monthly trading classes** (e.g. `2NVDA` row seen) — choosing trading_class==symbol avoids odd classes.

### Likely safe assumptions
- UTC date for DTE (≤1 day skew) — irrelevant to a 21–45 window.
- `time::Date` construction from YYYYMMDD via `Date::from_calendar_date`.

### Questions worth asking now
- None blocking.

## PRD ⚠️ resolutions
- D8 ⇒ SETTLED 📖 (ownership verified); cap 50 fixed.
- D9 ⇒ SETTLED: `pick_spot` precedence + Delayed* stripping pinned.
- D10 ⇒ SETTLED: usable = bid>0 && ask>0; rank/filter on credit_mid.
- Refinement vs PRD scope sketch: two-pass fetch (ADR 0039) replaces "all puts in [0.70,1.0]" to meet the PRD time goal.

## Card plan (hint for pipeline-task)
ONE card `01-pcs-scan`: spec-paths `tests/pcs_scan.rs`, `tests/help_command.rs` (re-freeze: +`"pcs-scan"`);
impl-paths `src/ib/pcs_scan.rs`, `src/ib/mod.rs`, `src/cli.rs`, `src/main.rs`, `src/surface.rs`.
Stub-compile + clippy the spec on scratch before freezing.
