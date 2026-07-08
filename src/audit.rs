//! Invocation audit log — append-only JSONL at the dispatch seam (ADR 0036).
//!
//! One line per successfully PARSED `omi` invocation (success AND failure),
//! written AFTER `run()` returns, BEFORE emit/exit. Write failure is fail-open
//! (ADR 0037): one `warn:` line to stderr, never the JSON envelope, never a
//! changed exit code.

use std::path::PathBuf;

use serde::Serialize;

/// `$HOME/.local/share/oh-my-ib/invocations.jsonl` — resolved exactly like
/// `Config::config_path()` (plain `var_os("HOME")` + join, no dirs crate).
/// HOME-derived so tests override `HOME` to a temp dir and stay hermetic.
pub fn log_path() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|home| {
        PathBuf::from(home).join(".local/share/oh-my-ib/invocations.jsonl")
    })
}

/// One audit record (ADR 0036 schema; additive-only evolution).
#[derive(Serialize)]
pub struct AuditEntry {
    /// RFC3339 UTC timestamp.
    pub ts: String,
    /// The command name from `surface::command_name()` — the shared anchor.
    pub cmd: &'static str,
    /// `std::env::args().skip(1)` with the `--account` value redacted to `"***"`.
    pub argv: Vec<String>,
    /// `"live"` iff `--live` was passed, else `"paper"`.
    pub mode: &'static str,
    /// `GlobalOpts.preview`.
    pub preview: bool,
    /// The process exit code about to be used.
    pub exit: i32,
    /// `AppError::code()` when failed, `null` when ok.
    pub error: Option<&'static str>,
    /// Wall-clock duration of the invocation in milliseconds.
    pub duration_ms: u64,
}

/// RFC3339 UTC timestamp via the `time` crate (existing dep; `formatting`
/// feature enabled transitively via `ibapi`).
pub fn rfc3339_utc() -> String {
    use time::format_description::well_known::Rfc3339;
    time::OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "unknown".to_string())
}

/// `std::env::args().skip(1)` with the value following `--account` redacted.
/// Handles both `--account X` (two args) and `--account=X` (one arg) forms.
/// Redacts the LOGGED argv only — never alters the real CLI parsing.
pub fn redacted_argv() -> Vec<String> {
    let mut argv: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < argv.len() {
        if argv[i] == "--account" {
            if i + 1 < argv.len() {
                argv[i + 1] = "***".to_string();
                i += 2;
                continue;
            }
        } else if argv[i].starts_with("--account=") {
            argv[i] = "--account=***".to_string();
        }
        i += 1;
    }
    argv
}

/// Append one JSON line to the audit log. `create_dir_all`s the parent on first
/// write. Returns `Err` on I/O failure — the caller applies fail-open (ADR 0037).
pub fn append(entry: &AuditEntry) -> std::io::Result<()> {
    use std::io::Write;
    let path = log_path()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "HOME not set"))?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let line = serde_json::to_string(entry)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    writeln!(file, "{line}")?;
    Ok(())
}

/// Read the last `n` parsed entries (newest last) + count of skipped malformed
/// lines across the ENTIRE file. Missing file ⇒ `(vec![], 0)`. A malformed or
/// truncated line is skipped AND counted, never fatal.
pub fn read_tail(n: usize) -> (Vec<serde_json::Value>, usize) {
    let path = match log_path() {
        Some(p) => p,
        None => return (vec![], 0),
    };
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(_) => return (vec![], 0),
    };
    let mut entries: Vec<serde_json::Value> = Vec::new();
    let mut skipped = 0usize;
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<serde_json::Value>(line) {
            Ok(v) => entries.push(v),
            Err(_) => skipped += 1,
        }
    }
    let start = entries.len().saturating_sub(n);
    (entries[start..].to_vec(), skipped)
}
