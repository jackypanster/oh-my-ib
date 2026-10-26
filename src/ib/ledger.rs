//! `ledger` — N-day cash ledger (deposits and other cash movements). READ ONLY.
//!
//! ibapi 3.1 `Client` has no cash-transaction request and no Flex download. The
//! closest socket call is `reqAccountSummary` with group `All` and tag `$LEDGER`
//! (`AccountSummaryTags::LEDGER`, TWS/Gateway 956+). IB documents that tag as a
//! **currency balance** snapshot (`CashBalance` / `TotalCashBalance`), not a dated
//! deposit list. This command probes that call (bounded), then sets `coverage`:
//!
//! - `supported` — a dated cash-transaction feed answered. Empty `entries` means
//!   none in the window (`empty_means: "no_activity"`).
//! - `partial` — that feed answered incompletely (timeout, error, undated rows).
//! - `unsupported` — no dated feed. `$LEDGER` rows are counted under `probe` and
//!   are not copied into `entries`. Exit 0, `empty_means: "feed_missing"`, message
//!   names Deferred Flex. Never a silent empty success.
//!
//! The live probe cannot flip `cash_transaction_feed` on: balances are not
//! deposits. Does not call `place_order` / `cancel_order` (`trade.rs` only).

use std::collections::BTreeSet;
use std::time::Instant;

use ibapi::accounts::types::AccountGroup;
use ibapi::accounts::{AccountSummaryResult, AccountSummaryTags};
use ibapi::client::blocking::Client;
use ibapi::prelude::SubscriptionItem;
use serde_json::{json, Value};
use time::Date;

use crate::config::Config;
use crate::error::AppError;

/// Default lookback. Kept in sync with the clap `--days` default.
pub const DEFAULT_DAYS: u32 = 7;
/// Inclusive maximum lookback.
pub const MAX_DAYS: u32 = 90;

const SOURCE_PROBE: &str = "probe:reqAccountSummary:$LEDGER";
const SOURCE_FEED: &str = "cash-transactions";
const PROBE_TAG: &str = AccountSummaryTags::LEDGER;

/// One dated cash movement. `type_name` serializes as JSON `type`.
#[derive(Debug, Clone, PartialEq)]
pub struct LedgerEntry {
    pub time: String,
    pub amount: f64,
    pub currency: String,
    pub type_name: String,
    pub description: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedgerCoverage {
    Supported,
    Unsupported,
    Partial,
}

impl LedgerCoverage {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Supported => "supported",
            Self::Unsupported => "unsupported",
            Self::Partial => "partial",
        }
    }
}

/// Why a probe stopped. `Ended` / `Closed` are complete reads; the others are not.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ProbeStop {
    Ended,
    /// Stream ended without an `accountSummaryEnd` marker.
    #[default]
    Closed,
    TimedOut,
    Failed(String),
}

impl ProbeStop {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Ended => "ended",
            Self::Closed => "closed",
            Self::TimedOut => "timed_out",
            Self::Failed(_) => "failed",
        }
    }

    fn is_complete(&self) -> bool {
        matches!(self, Self::Ended | Self::Closed)
    }

    fn error(&self) -> Option<&str> {
        match self {
            Self::Failed(msg) => Some(msg.as_str()),
            _ => None,
        }
    }
}

/// Facts the pure classifier sees. The live probe always leaves
/// `cash_transaction_feed` false — `$LEDGER` never produces `transactions`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ProbeFacts {
    pub cash_transaction_feed: bool,
    pub balance_rows: usize,
    pub undated: usize,
    pub stop: ProbeStop,
    pub tags: BTreeSet<String>,
    pub transactions: Vec<LedgerEntry>,
}

/// `--days` must be 1..=[`MAX_DAYS`]. Failure is `usage` / exit 64, before connect.
pub fn validate_days(days: u32) -> Result<u32, AppError> {
    if (1..=MAX_DAYS).contains(&days) {
        Ok(days)
    } else {
        Err(AppError::usage(
            format!(
                "--days must be an integer from 1 to {MAX_DAYS} (got {days}); default is {DEFAULT_DAYS}"
            ),
            "ledger",
        ))
    }
}

/// Keep rows whose UTC date falls in the inclusive window `[today-(days-1), today]`.
/// Unparseable timestamps count as `undated` (they are not silent drops).
pub fn select_in_window(
    entries: Vec<LedgerEntry>,
    days: u32,
    today: Date,
) -> (Vec<LedgerEntry>, usize) {
    let cutoff = cutoff_date(today, days);
    let mut kept = Vec::new();
    let mut undated = 0usize;
    for entry in entries {
        if !entry.amount.is_finite() {
            undated += 1;
            continue;
        }
        match parse_entry_date(&entry.time) {
            Some(date) if date >= cutoff && date <= today => kept.push(entry),
            Some(_) => {}
            None => undated += 1,
        }
    }
    (kept, undated)
}

pub fn classify_ledger_probe(facts: &ProbeFacts) -> LedgerCoverage {
    if !facts.cash_transaction_feed {
        // Balance rows, a clean end, or a probe error: still not a deposit ledger.
        return LedgerCoverage::Unsupported;
    }
    let incomplete = facts.undated > 0 || !facts.stop.is_complete();
    if incomplete {
        LedgerCoverage::Partial
    } else {
        LedgerCoverage::Supported
    }
}

pub fn coverage_message(
    coverage: LedgerCoverage,
    days: u32,
    entries_empty: bool,
    probe_error: Option<&str>,
) -> String {
    match coverage {
        LedgerCoverage::Supported if entries_empty => {
            format!("no cash transactions in the last {days} days")
        }
        LedgerCoverage::Supported => {
            format!("dated cash transactions in the last {days} days")
        }
        LedgerCoverage::Partial => format!(
            "partial cash-transaction feed for the last {days} days; entries may be incomplete. Full history is a Deferred Flex Activity Query (CashTransactions)."
        ),
        LedgerCoverage::Unsupported => {
            let mut msg = format!(
                "TWS socket API (ibapi 3.1) has no N-day cash-transaction ledger. reqAccountSummary tag $LEDGER (group All) returns currency balances (CashBalance, TotalCashBalance), not dated deposits. entries is empty because the feed is unsupported, not because the last {days} days had no cash activity. Verify a deposit with a Deferred Flex Activity Query (CashTransactions)."
            );
            if let Some(err) = probe_error {
                msg.push_str(" Probe error: ");
                msg.push_str(&one_line(err));
            }
            msg
        }
    }
}

fn empty_means(coverage: LedgerCoverage, entries_empty: bool) -> &'static str {
    if !entries_empty {
        return "not_empty";
    }
    match coverage {
        LedgerCoverage::Supported => "no_activity",
        LedgerCoverage::Partial => "incomplete_feed",
        LedgerCoverage::Unsupported => "feed_missing",
    }
}

fn source_for(coverage: LedgerCoverage) -> &'static str {
    match coverage {
        LedgerCoverage::Supported | LedgerCoverage::Partial => SOURCE_FEED,
        LedgerCoverage::Unsupported => SOURCE_PROBE,
    }
}

/// JSON shape: `account`, `days`, `coverage`, `source`, `entries[]`, plus
/// `message` / `empty_means` / `probe` so an empty `entries` is never silent.
pub fn shape_ledger(
    account: &str,
    days: u32,
    coverage: LedgerCoverage,
    message: &str,
    facts: &ProbeFacts,
) -> Value {
    let entries: Vec<Value> = facts
        .transactions
        .iter()
        .map(|e| {
            json!({
                "time": e.time,
                "amount": e.amount,
                "currency": e.currency,
                "type": e.type_name,
                "description": e.description,
            })
        })
        .collect();
    let mut probe = json!({
        "request": "reqAccountSummary",
        "group": "All",
        "tag": PROBE_TAG,
        "native_cash_transactions": facts.cash_transaction_feed,
        "balance_rows": facts.balance_rows,
        "stop": facts.stop.as_str(),
        "tags": facts.tags.iter().cloned().collect::<Vec<_>>(),
    });
    if let Some(err) = facts.stop.error() {
        if let Some(obj) = probe.as_object_mut() {
            obj.insert("error".to_string(), Value::from(one_line(err)));
        }
    }
    json!({
        "account": account,
        "days": days,
        "coverage": coverage.as_str(),
        "source": source_for(coverage),
        "entries": entries,
        "message": message,
        "empty_means": empty_means(coverage, entries.is_empty()),
        "probe": probe,
    })
}

pub fn shape_from_probe(account: &str, days: u32, mut facts: ProbeFacts) -> Value {
    // No dated feed: drop anything a caller stuffed into `transactions` so a
    // balance probe cannot render deposit rows under coverage=unsupported.
    if !facts.cash_transaction_feed {
        facts.transactions.clear();
    }
    let today = time::OffsetDateTime::now_utc().date();
    let (kept, undated) = select_in_window(std::mem::take(&mut facts.transactions), days, today);
    facts.undated = facts.undated.saturating_add(undated);
    facts.transactions = kept;
    let coverage = classify_ledger_probe(&facts);
    let message = coverage_message(
        coverage,
        days,
        facts.transactions.is_empty(),
        facts.stop.error(),
    );
    shape_ledger(account, days, coverage, &message, &facts)
}

pub fn ledger(cfg: &Config, days: u32) -> Result<Value, AppError> {
    let days = validate_days(days)?;
    let client = super::connect(cfg)?;
    let account = super::resolve_account(&client, cfg)?;
    let facts = probe_cash_ledger(&client);
    Ok(shape_from_probe(&account.0, days, facts))
}

/// Bounded `reqAccountSummary` / `$LEDGER` probe. Any gateway error is recorded
/// on the facts; the caller still emits `coverage: "unsupported"` rather than an
/// empty success. Balance tag values are not retained.
fn probe_cash_ledger(client: &Client) -> ProbeFacts {
    let group = AccountGroup("All".to_string());
    let subscription = match client.account_summary(&group, &[PROBE_TAG]) {
        Ok(sub) => sub,
        Err(e) => {
            return ProbeFacts {
                stop: ProbeStop::Failed(e.to_string()),
                ..ProbeFacts::default()
            };
        }
    };

    let deadline = Instant::now() + super::TAKE_FIRST_TIMEOUT;
    let mut facts = ProbeFacts::default();
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            facts.stop = ProbeStop::TimedOut;
            break;
        }
        match subscription.next_timeout(remaining) {
            Some(Ok(SubscriptionItem::Data(AccountSummaryResult::Summary(row)))) => {
                // Count only. `$LEDGER` values are balances, not ledger lines.
                facts.balance_rows += 1;
                if !row.tag.is_empty() {
                    facts.tags.insert(row.tag);
                }
            }
            Some(Ok(SubscriptionItem::Data(AccountSummaryResult::End))) => {
                facts.stop = ProbeStop::Ended;
                break;
            }
            Some(Ok(SubscriptionItem::Notice(_))) => {}
            Some(Err(e)) => {
                facts.stop = ProbeStop::Failed(e.to_string());
                break;
            }
            None if Instant::now() >= deadline => {
                facts.stop = ProbeStop::TimedOut;
                break;
            }
            None => {
                facts.stop = ProbeStop::Closed;
                break;
            }
        }
    }
    // Drop cancels the summary subscription (IB allows two at a time).
    drop(subscription);
    // Invariant: this socket probe has no dated cash-transaction rows.
    facts.cash_transaction_feed = false;
    facts.transactions.clear();
    facts
}

fn cutoff_date(today: Date, days: u32) -> Date {
    let back = i64::from(days.saturating_sub(1));
    today
        .checked_sub(time::Duration::days(back))
        .unwrap_or(today)
}

/// `YYYY-MM-DD` prefix (ISO-8601 / `YYYY-MM-DD...`). Anything else is undated.
fn parse_entry_date(time: &str) -> Option<Date> {
    let bytes = time.as_bytes();
    if bytes.len() < 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return None;
    }
    let year: i32 = time[0..4].parse().ok()?;
    let month: u8 = time[5..7].parse().ok()?;
    let day: u8 = time[8..10].parse().ok()?;
    let month = time::Month::try_from(month).ok()?;
    Date::from_calendar_date(year, month, day).ok()
}

fn one_line(s: &str) -> String {
    s.replace(['\n', '\r'], " ").chars().take(400).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn entry(time: &str, amount: f64) -> LedgerEntry {
        LedgerEntry {
            time: time.to_string(),
            amount,
            currency: "USD".to_string(),
            type_name: "Deposit".to_string(),
            description: "wire".to_string(),
        }
    }

    fn feed(transactions: Vec<LedgerEntry>, stop: ProbeStop) -> ProbeFacts {
        ProbeFacts {
            cash_transaction_feed: true,
            stop,
            transactions,
            ..ProbeFacts::default()
        }
    }

    #[test]
    fn days_bounds_are_1_to_90() {
        assert_eq!(DEFAULT_DAYS, 7);
        assert_eq!(MAX_DAYS, 90);
        assert!(validate_days(1).is_ok());
        assert!(validate_days(7).is_ok());
        assert!(validate_days(90).is_ok());
        for bad in [0, 91, 365] {
            let err = validate_days(bad).unwrap_err();
            assert_eq!(err.code(), "usage", "days={bad}");
            assert_eq!(err.exit_code(), 64);
            assert!(err.message.contains("1 to 90"));
        }
    }

    #[test]
    fn window_keeps_inclusive_cutoff_and_counts_undated() {
        let today = Date::from_calendar_date(2026, time::Month::October, 26).unwrap();
        let (kept, undated) = select_in_window(
            vec![
                entry("2026-10-20T00:00:00Z", 200.0),
                entry("2026-10-19T23:59:59Z", 50.0),
                entry("not-a-time", 1.0),
                entry("2026-10-27T00:00:00Z", 9.0),
                entry("2026-10-26T15:00:00Z", f64::NAN),
            ],
            7,
            today,
        );
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].amount, 200.0);
        assert_eq!(undated, 2);
    }

    #[test]
    fn supported_entry_shape_includes_time_amount_currency_type() {
        let today = time::OffsetDateTime::now_utc().date();
        let stamp = format!("{today}T12:00:00Z");
        let v = shape_from_probe(
            "ACCT",
            14,
            feed(vec![entry(&stamp, 200.0)], ProbeStop::Ended),
        );
        assert_eq!(v["account"], json!("ACCT"));
        assert_eq!(v["days"], json!(14));
        assert_eq!(v["coverage"], json!("supported"));
        assert_eq!(v["source"], json!(SOURCE_FEED));
        assert_eq!(v["empty_means"], json!("not_empty"));
        let row = &v["entries"][0];
        assert_eq!(row["time"], json!(stamp));
        assert_eq!(row["amount"], json!(200.0));
        assert_eq!(row["currency"], json!("USD"));
        assert_eq!(row["type"], json!("Deposit"));
        assert_eq!(row["description"], json!("wire"));
        assert!(!v["message"].as_str().unwrap().contains("Deferred Flex"));
    }

    #[test]
    fn empty_supported_means_no_activity_in_window() {
        let v = shape_from_probe("ACCT", 7, feed(vec![], ProbeStop::Ended));
        assert_eq!(v["coverage"], json!("supported"));
        assert!(v["entries"].as_array().unwrap().is_empty());
        assert_eq!(v["empty_means"], json!("no_activity"));
        let msg = v["message"].as_str().unwrap();
        assert!(msg.contains("no cash transactions"));
        assert!(!msg.contains("Deferred Flex"));
        assert!(!msg.contains("unsupported"));
    }

    #[test]
    fn balance_probe_is_unsupported_not_silent_empty() {
        let mut facts = ProbeFacts {
            balance_rows: 2,
            stop: ProbeStop::Ended,
            ..ProbeFacts::default()
        };
        facts.tags.insert("CashBalance".to_string());
        facts.tags.insert("TotalCashBalance".to_string());
        // A balance snapshot must not be rewritten as a deposit row.
        facts
            .transactions
            .push(entry("2026-10-26T00:00:00Z", 200.0));
        let v = shape_from_probe("ACCT", 7, facts);
        assert_eq!(v["coverage"], json!("unsupported"));
        assert!(
            v["entries"].as_array().unwrap().is_empty(),
            "balances and stray rows are not a ledger when the feed flag is false"
        );
        assert_eq!(v["empty_means"], json!("feed_missing"));
        assert_eq!(v["source"], json!(SOURCE_PROBE));
        let msg = v["message"].as_str().unwrap();
        assert!(msg.contains("Deferred Flex"), "{msg}");
        assert!(msg.contains("unsupported"), "{msg}");
        assert_eq!(v["probe"]["native_cash_transactions"], json!(false));
        assert_eq!(v["probe"]["balance_rows"], json!(2));
        assert_eq!(v["probe"]["tag"], json!("$LEDGER"));
        assert_ne!(v["coverage"], json!("supported"));
        assert_ne!(v["empty_means"], json!("no_activity"));
    }

    #[test]
    fn probe_failure_stays_unsupported_and_names_the_error() {
        let v = shape_from_probe(
            "ACCT",
            7,
            ProbeFacts {
                stop: ProbeStop::Failed("group rejected".into()),
                ..ProbeFacts::default()
            },
        );
        assert_eq!(v["coverage"], json!("unsupported"));
        assert_eq!(v["empty_means"], json!("feed_missing"));
        assert!(v["entries"].as_array().unwrap().is_empty());
        assert!(v["message"]
            .as_str()
            .unwrap()
            .contains("Probe error: group rejected"));
        assert!(v["message"].as_str().unwrap().contains("Deferred Flex"));
        assert_eq!(v["probe"]["stop"], json!("failed"));
    }

    #[test]
    fn undated_rows_on_a_real_feed_are_partial_not_empty_supported() {
        let v = shape_from_probe(
            "ACCT",
            7,
            feed(vec![entry("not-a-time", 200.0)], ProbeStop::Ended),
        );
        assert_eq!(v["coverage"], json!("partial"));
        assert!(v["entries"].as_array().unwrap().is_empty());
        assert_eq!(v["empty_means"], json!("incomplete_feed"));
        assert!(v["message"].as_str().unwrap().contains("Deferred Flex"));
        assert_ne!(v["coverage"], json!("supported"));
        assert_ne!(v["empty_means"], json!("no_activity"));
    }

    #[test]
    fn outside_window_on_a_real_feed_is_empty_supported() {
        let v = shape_from_probe(
            "ACCT",
            7,
            feed(vec![entry("2000-01-01T00:00:00Z", 200.0)], ProbeStop::Ended),
        );
        assert_eq!(v["coverage"], json!("supported"));
        assert!(v["entries"].as_array().unwrap().is_empty());
        assert_eq!(v["empty_means"], json!("no_activity"));
    }
}
