//! FROZEN SPEC — close-pending-guard card 01 (ADR 0023). Offline. The coder must NOT edit
//! this file.
//!
//! Freezes the pure blocking seam `blocking_close_order_ids(position, conid, open_orders)`:
//! working orders that BLOCK a close = SAME conid AND action OPPOSITE to the position sign
//! (long ⇒ "Sell" blocks, short ⇒ "Buy" blocks; action strings are the Debug forms the
//! shared drain emits, orders.rs precedent). Same-side orders (adds) never block; other
//! conids never block; ids return ASCENDING; position 0 ⇒ empty (defensive totality — the
//! verb refuses flat positions upstream). RED until impl re-exports
//! `oh_my_ib::ib::blocking_close_order_ids`.
//!
//! NOT frozen (reviewed-by-reading + deferred paper lifecycle, PRD criterion 7): the guard
//! wiring inside `option_close` (drain reuse on the same client, ordering after the non-OPT
//! check and before derive_close, the not_found refusal naming ids, fail-closed row parse)
//! and the AGENTS.md phrase. Existing frozen suites (option_close_command.rs,
//! positions_row.rs) are untouched by this feature.

use oh_my_ib::ib::blocking_close_order_ids;

/// (order_id, conid, action) triples in the drain's Debug-string action form.
fn orders(list: &[(i32, i32, &str)]) -> Vec<(i32, i32, String)> {
    list.iter().map(|(id, conid, a)| (*id, *conid, a.to_string())).collect()
}

const CONID: i32 = 495512569;

#[test]
fn long_position_with_working_sell_on_same_conid_blocks() {
    let open = orders(&[(101, CONID, "Sell")]);
    assert_eq!(blocking_close_order_ids(2.0, CONID, &open), vec![101]);
}

#[test]
fn long_position_with_working_buy_is_an_add_not_a_close() {
    let open = orders(&[(102, CONID, "Buy")]);
    assert_eq!(blocking_close_order_ids(2.0, CONID, &open), Vec::<i32>::new());
}

#[test]
fn short_position_with_working_buy_on_same_conid_blocks() {
    let open = orders(&[(103, CONID, "Buy")]);
    assert_eq!(blocking_close_order_ids(-2.0, CONID, &open), vec![103]);
}

#[test]
fn short_position_with_working_sell_does_not_block() {
    let open = orders(&[(104, CONID, "Sell")]);
    assert_eq!(blocking_close_order_ids(-2.0, CONID, &open), Vec::<i32>::new());
}

#[test]
fn other_conid_orders_never_block() {
    let open = orders(&[(105, 99, "Sell"), (106, 100, "Buy")]);
    assert_eq!(blocking_close_order_ids(2.0, CONID, &open), Vec::<i32>::new());
}

#[test]
fn multiple_blockers_return_all_ids_ascending() {
    // Mixed bag: two blockers out of order, one same-side add, one other-conid order.
    let open = orders(&[
        (300, CONID, "Sell"),
        (100, CONID, "Sell"),
        (200, CONID, "Buy"),
        (400, 99, "Sell"),
    ]);
    assert_eq!(blocking_close_order_ids(2.0, CONID, &open), vec![100, 300]);
}

#[test]
fn zero_position_returns_empty_defensive_totality() {
    // Unreachable in the verb (flat is refused upstream) — the seam is still total.
    let open = orders(&[(107, CONID, "Sell"), (108, CONID, "Buy")]);
    assert_eq!(blocking_close_order_ids(0.0, CONID, &open), Vec::<i32>::new());
}

#[test]
fn empty_order_book_never_blocks() {
    assert_eq!(blocking_close_order_ids(2.0, CONID, &[]), Vec::<i32>::new());
}
