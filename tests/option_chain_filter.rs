//! FROZEN SPEC — option-chain-default-exchange (card 01): the pure client-side exchange
//! filter seam `filter_chain_rows` (ADR 0028). Offline. The coder must NOT edit this file.
//!
//! Freezes `filter_chain_rows(rows: Vec<ChainRow>, exchange: &str) -> Vec<ChainRow>`. Rules:
//!
//! - `""` is passthrough — every row is returned, input order preserved.
//! - a non-empty `<EX>` retains only rows whose `exchange` equals it (exact string, case sensitive).
//! - all matching rows are kept (never deduped); the retained subset keeps its input order.
//! - no matching row yields an empty vec, so upstream `shape_option_chain` returns honest `chains: []`.
//!
//! RED until impl adds `filter_chain_rows` and re-exports `oh_my_ib::ib::filter_chain_rows` (the
//! import below will not resolve until then).
//!
//! NOT frozen (reviewed-by-reading + operator LIVE acceptance, PRD criteria 1-3): the gateway-fn
//! wiring (reqSecDefOptParams called with `""` always, and the seam insertion point between the drain
//! and `shape_option_chain`), the CLI `--exchange` help text, and the doc comments. Tiger `:4001`
//! behavior (server-side SMART yields empty; `""` yields 20 identical rows including a SMART row) is a
//! journaled observation, never asserted here. The `shape_option_chain` seam is a SEPARATE frozen spec
//! (tests/option_chain_command.rs) and is UNCHANGED by this feature.

use oh_my_ib::ib::{filter_chain_rows, ChainRow};

fn row(exchange: &str) -> ChainRow {
    ChainRow {
        exchange: exchange.to_string(),
        trading_class: "AAPL".to_string(),
        multiplier: "100".to_string(),
        expirations: vec!["20260117".to_string()],
        strikes: vec![100.0],
    }
}

fn exchanges(rows: &[ChainRow]) -> Vec<String> {
    rows.iter().map(|r| r.exchange.clone()).collect()
}

#[test]
fn empty_exchange_is_passthrough_preserving_order() {
    let rows = vec![row("AMEX"), row("SMART"), row("CBOE")];
    let out = filter_chain_rows(rows, "");
    assert_eq!(
        exchanges(&out),
        vec!["AMEX", "SMART", "CBOE"],
        "empty exchange = all rows, input order preserved"
    );
}

#[test]
fn smart_retains_only_the_smart_row() {
    let rows = vec![row("AMEX"), row("SMART"), row("CBOE"), row("PHLX")];
    let out = filter_chain_rows(rows, "SMART");
    assert_eq!(
        exchanges(&out),
        vec!["SMART"],
        "default SMART filter = the single consolidated SMART row (the clean out-of-box view)"
    );
}

#[test]
fn named_exchange_retains_only_that_exchange() {
    let rows = vec![row("AMEX"), row("SMART"), row("CBOE")];
    let out = filter_chain_rows(rows, "AMEX");
    assert_eq!(exchanges(&out), vec!["AMEX"]);
}

#[test]
fn no_matching_exchange_yields_empty() {
    let rows = vec![row("AMEX"), row("SMART")];
    let out = filter_chain_rows(rows, "NASDAQ_NOPE");
    assert!(
        out.is_empty(),
        "an exchange present in no row yields empty, so upstream chains:[] (honest empty, not a crash)"
    );
}

#[test]
fn match_is_exact_string_case_sensitive() {
    let rows = vec![row("SMART")];
    let out = filter_chain_rows(rows, "smart");
    assert!(
        out.is_empty(),
        "exact string, case sensitive: 'smart' must NOT match 'SMART'"
    );
}

#[test]
fn retains_all_matching_rows_no_dedup_order_preserved() {
    // Two SMART rows (e.g. different trading_class in reality) — both retained, input order kept.
    let rows = vec![row("CBOE"), row("SMART"), row("AMEX"), row("SMART")];
    let out = filter_chain_rows(rows, "SMART");
    assert_eq!(out.len(), 2, "all SMART rows retained (the seam filters, never dedups)");
    assert_eq!(exchanges(&out), vec!["SMART", "SMART"]);
}
