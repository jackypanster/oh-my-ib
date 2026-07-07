//! FROZEN SPEC — grid-tick (ADR 0033): the pure mean-reversion grid planner + config parse.
//! Offline. The coder must NOT edit this file.
//!
//! Freezes `plan_grid_tick(cfg, acct, positions, open) -> Vec<Action>` — the pure reconcile-to-desired
//! -state planner (no client, no I/O) — and `GridConfig::from_toml_str(&str) -> Result<_, AppError>`
//! (parse + serde-defaults + validation). Per configured symbol with a cost anchor (`qty>0`): desire a
//! Sell `min(lot,qty)` @ `round2(avg_cost*(1+rise%))` and, IFF cash is above the net-liq floor AND
//! `qty+lot<=max_shares`, a Buy `lot` @ `round2(avg_cost*(1-drop%))`; reconcile against existing orders
//! (match = same side ∧ same qty ∧ |Δlimit|<=0.005 ⇒ keep, else cancel+replace); a flat symbol (`qty==0`)
//! cancels its lingering orders and places nothing; symbols NOT in the config are never touched. Output
//! orders all Cancels before all Places.
//!
//! RED until impl adds `src/grid.rs` (`pub mod grid;` in `src/lib.rs`) with these exact types + the
//! planner + the config parse — the `oh_my_ib::grid::*` import below will not resolve until then.
//!
//! NOT frozen (reviewed-by-reading, ADR 0033 §Freeze-coverage): the gateway driver `src/ib/grid.rs`
//! (single `account_updates` drain feeding cash+positions; `all_open_orders`; Action→execution on the
//! shared client, Cancels-first, stop-on-first-error, no blind retry; the live-refusal guard; the JSON
//! envelope; the `pub(crate)` extractions leaving `build_stk_order`/`cancel`/`place` byte-identical).
//! NOT frozen (operator acceptance): the paper `:4002` lifecycle in ADR 0033.

use std::collections::HashMap;

use oh_my_ib::grid::{
    plan_grid_tick, Action, AccountSnap, GridConfig, OpenOrderLite, PositionLite, Side, SymbolCfg,
};

// ---- test builders ---------------------------------------------------------

fn scfg(name: &str, drop_pct: f64, rise_pct: f64, max_shares: u32) -> SymbolCfg {
    SymbolCfg { name: name.to_string(), drop_pct, rise_pct, max_shares }
}

fn gcfg(lot: u32, cash_floor_pct: f64, symbols: Vec<SymbolCfg>) -> GridConfig {
    GridConfig { lot, cash_floor_pct, symbols }
}

fn acct(total_cash: f64, net_liquidation: f64) -> AccountSnap {
    AccountSnap { total_cash, net_liquidation }
}

fn positions(items: &[(&str, f64, f64)]) -> HashMap<String, PositionLite> {
    items
        .iter()
        .map(|(s, qty, avg)| (s.to_string(), PositionLite { qty: *qty, avg_cost: *avg }))
        .collect()
}

fn oo(order_id: i32, symbol: &str, side: Side, limit: f64, qty: f64) -> OpenOrderLite {
    OpenOrderLite { order_id, symbol: symbol.to_string(), side, limit, qty }
}

// ---- assertion helpers (avoid clippy::float_cmp under -D warnings) ----------

fn approx(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

/// The (qty, limit) of the Place for `symbol`/`side`, if the plan contains exactly one.
fn place_of(actions: &[Action], symbol: &str, side: Side) -> Option<(f64, f64)> {
    actions.iter().find_map(|a| match a {
        Action::Place { symbol: s, side: sd, qty, limit } if s == symbol && *sd == side => {
            Some((*qty, *limit))
        }
        _ => None,
    })
}

fn cancel_ids(actions: &[Action]) -> Vec<i32> {
    actions
        .iter()
        .filter_map(|a| match a {
            Action::Cancel { order_id } => Some(*order_id),
            _ => None,
        })
        .collect()
}

fn place_count(actions: &[Action]) -> usize {
    actions.iter().filter(|a| matches!(a, Action::Place { .. })).count()
}

// ============================================================================
// (a) held symbol, no existing orders ⇒ one Buy@-drop% + one Sell@+rise%
// ============================================================================

#[test]
fn held_symbol_places_buy_and_sell_pair() {
    let cfg = gcfg(100, 50.0, vec![scfg("NVDA", 2.0, 2.0, 300)]);
    let plan = plan_grid_tick(&cfg, &acct(100_000.0, 100_000.0), &positions(&[("NVDA", 100.0, 100.0)]), &[]);

    let (bq, bl) = place_of(&plan, "NVDA", Side::Buy).expect("a Buy rung is desired");
    let (sq, sl) = place_of(&plan, "NVDA", Side::Sell).expect("a Sell rung is desired");
    assert!(approx(bq, 100.0), "buy qty = lot (100), got {bq}");
    assert!(approx(bl, 98.0), "buy @ round2(100*(1-0.02)) = 98.00, got {bl}");
    assert!(approx(sq, 100.0), "sell qty = min(lot,qty) = 100, got {sq}");
    assert!(approx(sl, 102.0), "sell @ round2(100*(1+0.02)) = 102.00, got {sl}");
    assert!(cancel_ids(&plan).is_empty(), "no existing orders ⇒ no cancels");
    assert_eq!(place_count(&plan), 2, "exactly the buy+sell pair");
}

// ============================================================================
// (b) cash below the net-liq floor ⇒ NO Buy, Sell still present
// ============================================================================

#[test]
fn cash_below_floor_suppresses_buy_only() {
    let cfg = gcfg(100, 50.0, vec![scfg("NVDA", 2.0, 2.0, 300)]);
    // 40k cash < 50% of 100k net-liq ⇒ buy suppressed.
    let plan = plan_grid_tick(&cfg, &acct(40_000.0, 100_000.0), &positions(&[("NVDA", 100.0, 100.0)]), &[]);

    assert!(place_of(&plan, "NVDA", Side::Buy).is_none(), "cash below floor ⇒ no buy");
    let (sq, sl) = place_of(&plan, "NVDA", Side::Sell).expect("sell is never cash-gated");
    assert!(approx(sq, 100.0) && approx(sl, 102.0));
    assert_eq!(place_count(&plan), 1, "only the sell");
}

// ============================================================================
// (c) max_shares is a STRICT never-exceed ceiling (qty + lot <= max_shares)
// ============================================================================

#[test]
fn max_shares_at_ceiling_suppresses_buy() {
    // qty=300, lot=100, max=300 ⇒ 300+100=400 > 300 ⇒ no buy.
    let cfg = gcfg(100, 50.0, vec![scfg("NVDA", 2.0, 2.0, 300)]);
    let plan = plan_grid_tick(&cfg, &acct(1_000_000.0, 1_000_000.0), &positions(&[("NVDA", 300.0, 100.0)]), &[]);
    assert!(place_of(&plan, "NVDA", Side::Buy).is_none(), "qty+lot>max ⇒ no buy");
    assert!(place_of(&plan, "NVDA", Side::Sell).is_some(), "sell still ladders down");
}

#[test]
fn max_shares_non_lot_multiple_never_overshoots() {
    // qty=200, lot=100, max=250 ⇒ 200+100=300 > 250 ⇒ no buy (strict form; a `<` test would overshoot).
    let cfg = gcfg(100, 50.0, vec![scfg("NVDA", 2.0, 2.0, 250)]);
    let plan = plan_grid_tick(&cfg, &acct(1_000_000.0, 1_000_000.0), &positions(&[("NVDA", 200.0, 100.0)]), &[]);
    assert!(place_of(&plan, "NVDA", Side::Buy).is_none(), "200+100>250 ⇒ no buy");
}

#[test]
fn max_shares_with_room_allows_buy() {
    // qty=200, lot=100, max=300 ⇒ 200+100=300 <= 300 ⇒ buy allowed (boundary is inclusive).
    let cfg = gcfg(100, 50.0, vec![scfg("NVDA", 2.0, 2.0, 300)]);
    let plan = plan_grid_tick(&cfg, &acct(1_000_000.0, 1_000_000.0), &positions(&[("NVDA", 200.0, 100.0)]), &[]);
    let (bq, bl) = place_of(&plan, "NVDA", Side::Buy).expect("200+100<=300 ⇒ buy allowed");
    assert!(approx(bq, 100.0) && approx(bl, 98.0));
}

// ============================================================================
// (d) flat symbol (qty==0, no anchor) ⇒ cancel lingering, place nothing
// ============================================================================

#[test]
fn flat_symbol_cancels_lingering_order() {
    let cfg = gcfg(100, 50.0, vec![scfg("NVDA", 2.0, 2.0, 300)]);
    // NVDA absent from positions ⇒ qty 0 ⇒ idle; a stale resting buy must be cancelled.
    let plan = plan_grid_tick(
        &cfg,
        &acct(1_000_000.0, 1_000_000.0),
        &positions(&[]),
        &[oo(5, "NVDA", Side::Buy, 98.0, 100.0)],
    );
    assert_eq!(cancel_ids(&plan), vec![5], "flat ⇒ cancel the lingering order");
    assert_eq!(place_count(&plan), 0, "flat ⇒ no new orders (idle until manual re-seed)");
}

#[test]
fn flat_symbol_with_no_orders_is_empty_plan() {
    let cfg = gcfg(100, 50.0, vec![scfg("NVDA", 2.0, 2.0, 300)]);
    let plan = plan_grid_tick(&cfg, &acct(1_000_000.0, 1_000_000.0), &positions(&[]), &[]);
    assert!(plan.is_empty(), "flat + no orders ⇒ nothing to do");
}

// ============================================================================
// (e) reconcile idempotence + drift
// ============================================================================

#[test]
fn already_matching_orders_produce_empty_plan() {
    let cfg = gcfg(100, 50.0, vec![scfg("NVDA", 2.0, 2.0, 300)]);
    // Existing buy@98/sell@102 already equal desired ⇒ zero actions (a pure-read tick).
    let plan = plan_grid_tick(
        &cfg,
        &acct(1_000_000.0, 1_000_000.0),
        &positions(&[("NVDA", 100.0, 100.0)]),
        &[oo(1, "NVDA", Side::Buy, 98.0, 100.0), oo(2, "NVDA", Side::Sell, 102.0, 100.0)],
    );
    assert!(plan.is_empty(), "matching orders ⇒ idempotent no-op, got {plan:?}");
}

#[test]
fn drifted_order_is_cancelled_then_replaced_cancels_first() {
    let cfg = gcfg(100, 50.0, vec![scfg("NVDA", 2.0, 2.0, 300)]);
    // Buy drifted (stale 90.0 vs desired 98.0); sell still matches ⇒ only the buy re-prices.
    let plan = plan_grid_tick(
        &cfg,
        &acct(1_000_000.0, 1_000_000.0),
        &positions(&[("NVDA", 100.0, 100.0)]),
        &[oo(1, "NVDA", Side::Buy, 90.0, 100.0), oo(2, "NVDA", Side::Sell, 102.0, 100.0)],
    );
    assert_eq!(cancel_ids(&plan), vec![1], "the drifted buy is cancelled; the matching sell is kept");
    let (bq, bl) = place_of(&plan, "NVDA", Side::Buy).expect("a fresh buy is placed at the new price");
    assert!(approx(bq, 100.0) && approx(bl, 98.0));
    assert!(place_of(&plan, "NVDA", Side::Sell).is_none(), "the matching sell is NOT re-placed");
    // Ordering invariant: every Cancel precedes every Place.
    let first_place = plan.iter().position(|a| matches!(a, Action::Place { .. })).unwrap();
    let last_cancel = plan.iter().rposition(|a| matches!(a, Action::Cancel { .. })).unwrap();
    assert!(last_cancel < first_place, "all Cancels must precede all Places: {plan:?}");
}

// ============================================================================
// (f) per-symbol independence; orders on UNconfigured symbols are never touched
// ============================================================================

#[test]
fn symbols_are_independent_and_unconfigured_untouched() {
    let cfg = gcfg(100, 50.0, vec![scfg("NVDA", 2.0, 2.0, 300), scfg("AAPL", 1.0, 3.0, 500)]);
    let plan = plan_grid_tick(
        &cfg,
        &acct(1_000_000.0, 1_000_000.0),
        &positions(&[("NVDA", 100.0, 100.0), ("AAPL", 100.0, 200.0), ("TSLA", 100.0, 50.0)]),
        // A resting order on TSLA, which is NOT in the config — must be invisible to the planner.
        &[oo(9, "TSLA", Side::Sell, 999.0, 100.0)],
    );

    let (_, nbl) = place_of(&plan, "NVDA", Side::Buy).expect("NVDA buy");
    let (_, nsl) = place_of(&plan, "NVDA", Side::Sell).expect("NVDA sell");
    assert!(approx(nbl, 98.0) && approx(nsl, 102.0), "NVDA uses its own 2%/2%");
    let (_, abl) = place_of(&plan, "AAPL", Side::Buy).expect("AAPL buy");
    let (_, asl) = place_of(&plan, "AAPL", Side::Sell).expect("AAPL sell");
    assert!(approx(abl, 198.0), "AAPL buy @ round2(200*(1-0.01)) = 198.00, got {abl}");
    assert!(approx(asl, 206.0), "AAPL sell @ round2(200*(1+0.03)) = 206.00, got {asl}");

    assert!(!cancel_ids(&plan).contains(&9), "the unconfigured-symbol order must NOT be cancelled");
    assert_eq!(place_count(&plan), 4, "two symbols × (buy+sell), nothing for TSLA");
}

// ============================================================================
// (g) GridConfig parse — defaults, overrides, and config-error on bad input
// ============================================================================

#[test]
fn config_parse_applies_defaults() {
    let cfg = GridConfig::from_toml_str("[[symbol]]\nname = \"NVDA\"\n").expect("minimal config parses");
    assert_eq!(cfg.lot, 100, "default lot = 100");
    assert!(approx(cfg.cash_floor_pct, 50.0), "default cash_floor_pct = 50");
    assert_eq!(cfg.symbols.len(), 1);
    let s = &cfg.symbols[0];
    assert_eq!(s.name, "NVDA");
    assert!(approx(s.drop_pct, 2.0), "default drop_pct = 2.0");
    assert!(approx(s.rise_pct, 2.0), "default rise_pct = 2.0");
    assert_eq!(s.max_shares, 300, "default max_shares = 300");
}

#[test]
fn config_parse_honors_overrides() {
    let toml = "lot = 200\ncash_floor_pct = 40\n\n[[symbol]]\nname = \"AAPL\"\ndrop_pct = 1.5\nrise_pct = 3.0\nmax_shares = 500\n";
    let cfg = GridConfig::from_toml_str(toml).expect("full config parses");
    assert_eq!(cfg.lot, 200);
    assert!(approx(cfg.cash_floor_pct, 40.0));
    let s = &cfg.symbols[0];
    assert!(approx(s.drop_pct, 1.5) && approx(s.rise_pct, 3.0));
    assert_eq!(s.max_shares, 500);
}

#[test]
fn config_negative_pct_is_config_error() {
    let err = GridConfig::from_toml_str("[[symbol]]\nname = \"NVDA\"\ndrop_pct = -2.0\n")
        .expect_err("a negative percentage is invalid");
    assert_eq!(err.code(), "config", "validation failure ⇒ code=config (exit 5)");
}

#[test]
fn config_empty_symbols_is_config_error() {
    let err = GridConfig::from_toml_str("lot = 100\n").expect_err("no symbols is invalid");
    assert_eq!(err.code(), "config");
}

#[test]
fn config_malformed_toml_is_config_error() {
    let err = GridConfig::from_toml_str("this is not = valid = toml").expect_err("bad toml is rejected");
    assert_eq!(err.code(), "config", "a toml parse error also maps to code=config");
}
