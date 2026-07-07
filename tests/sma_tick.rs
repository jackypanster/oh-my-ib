//! FROZEN SPEC — sma-tick (ADR 0035): the pure reconcile planner for the active 200-day timing executor.
//! Offline. The coder must NOT edit this file.
//!
//! Freezes `plan_sma_tick(state, current_qty, lot) -> TickAction`. Binary target: HOLD ⇒ `lot` shares,
//! EXIT ⇒ 0, INSUFFICIENT ⇒ don't trade. `delta = target - current_qty` → Buy(delta) / Sell(-delta) /
//! Noop. It reconciles from ANY current qty (0 → buy up to lot; lot → noop; over lot → sell down).
//!
//! RED until impl adds `src/ib/sma_tick.rs` (`pub mod`-exported at `src/ib/mod.rs`) with the TickAction
//! type + `plan_sma_tick` — the `oh_my_ib::ib::plan_sma_tick` import won't resolve until then.
//!
//! NOT frozen (review-by-reading, ADR 0035): the gateway `sma_tick_cmd` (paper-only guard, signal reuse
//! via `signal_for`, position read, the marketable-LMT price = latest_close×1.02/×0.98, execution via
//! build_stk_order + place_with_client, JSON, --dry-run) and the containment/read-only posture.

use oh_my_ib::ib::{plan_sma_tick, SignalState, TickAction};

fn approx(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

fn assert_buy(a: TickAction, want: f64) {
    match a {
        TickAction::Buy { qty } => assert!(approx(qty, want), "expected Buy {want}, got Buy {qty}"),
        other => panic!("expected Buy {want}, got {other:?}"),
    }
}

fn assert_sell(a: TickAction, want: f64) {
    match a {
        TickAction::Sell { qty } => assert!(approx(qty, want), "expected Sell {want}, got Sell {qty}"),
        other => panic!("expected Sell {want}, got {other:?}"),
    }
}

fn assert_noop(a: TickAction) {
    assert!(matches!(a, TickAction::Noop), "expected Noop, got {a:?}");
}

// ---- HOLD ⇒ target = lot -----------------------------------------------------

#[test]
fn hold_when_flat_buys_up_to_lot() {
    assert_buy(plan_sma_tick(SignalState::Hold, 0.0, 10.0), 10.0);
}

#[test]
fn hold_when_at_target_is_noop() {
    assert_noop(plan_sma_tick(SignalState::Hold, 10.0, 10.0));
}

#[test]
fn hold_when_partial_buys_the_difference() {
    assert_buy(plan_sma_tick(SignalState::Hold, 4.0, 10.0), 6.0);
}

#[test]
fn hold_when_over_target_sells_down() {
    // e.g. the operator manually holds 15; reconcile down to the 10-share target.
    assert_sell(plan_sma_tick(SignalState::Hold, 15.0, 10.0), 5.0);
}

// ---- EXIT ⇒ target = 0 -------------------------------------------------------

#[test]
fn exit_when_holding_sells_all() {
    assert_sell(plan_sma_tick(SignalState::Exit, 10.0, 10.0), 10.0);
}

#[test]
fn exit_when_flat_is_noop() {
    assert_noop(plan_sma_tick(SignalState::Exit, 0.0, 10.0));
}

// ---- INSUFFICIENT ⇒ never trade (no data) ------------------------------------

#[test]
fn insufficient_never_trades() {
    assert_noop(plan_sma_tick(SignalState::Insufficient, 0.0, 10.0));
    assert_noop(plan_sma_tick(SignalState::Insufficient, 10.0, 10.0));
}
