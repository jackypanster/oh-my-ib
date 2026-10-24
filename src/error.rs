//! Structured application error + JSON error envelope. Fast-fail: every error
//! carries a concrete message and (where known) the operation/target context.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// Could not reach / handshake with the IB Gateway.
    Connection,
    /// A bounded read produced no data in time (the message names the cause/cure).
    Timeout,
    /// A requested entity (contract, account) does not exist.
    NotFound,
    /// The gateway accepted the request but returned an error/notice, or a write
    /// precondition failed before anything was sent (order NOT sent).
    Data,
    /// Broker rejected a submitted order. The request was sent; it was not accepted.
    Rejected,
    /// A write was sent but the outcome is unknown (ack timeout, dropped connection).
    /// Callers must never auto-retry — verify with `omi orders`.
    Unconfirmed,
    /// Bad local config or flag combination. The live-port env gate (`OMI_ALLOW_LIVE`)
    /// stays here (exit 5); risk-cap blocks are [`ErrorKind::Guard`] (exit 64).
    Config,
    /// Local guard or risk-cap blocked a write before it was sent (exit 64, same as usage).
    Guard,
    /// Invalid command-line usage (argument parsing failure).
    Usage,
    /// Anything else.
    Other,
}

#[derive(Debug)]
pub struct AppError {
    pub kind: ErrorKind,
    pub message: String,
    pub context: Option<String>,
}

impl AppError {
    fn new(kind: ErrorKind, message: impl Into<String>, context: Option<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            context,
        }
    }
    pub fn connection(message: impl Into<String>, context: impl Into<String>) -> Self {
        Self::new(ErrorKind::Connection, message, Some(context.into()))
    }
    pub fn timeout(message: impl Into<String>, context: impl Into<String>) -> Self {
        Self::new(ErrorKind::Timeout, message, Some(context.into()))
    }
    pub fn not_found(message: impl Into<String>, context: impl Into<String>) -> Self {
        Self::new(ErrorKind::NotFound, message, Some(context.into()))
    }
    pub fn data(message: impl Into<String>, context: impl Into<String>) -> Self {
        Self::new(ErrorKind::Data, message, Some(context.into()))
    }
    pub fn rejected(message: impl Into<String>, context: impl Into<String>) -> Self {
        Self::new(ErrorKind::Rejected, message, Some(context.into()))
    }
    pub fn unconfirmed(message: impl Into<String>, context: impl Into<String>) -> Self {
        Self::new(ErrorKind::Unconfirmed, message, Some(context.into()))
    }
    pub fn config(message: impl Into<String>, context: impl Into<String>) -> Self {
        Self::new(ErrorKind::Config, message, Some(context.into()))
    }
    pub fn guard(message: impl Into<String>, context: impl Into<String>) -> Self {
        Self::new(ErrorKind::Guard, message, Some(context.into()))
    }
    pub fn usage(message: impl Into<String>, context: impl Into<String>) -> Self {
        Self::new(ErrorKind::Usage, message, Some(context.into()))
    }
    pub fn other(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Other, message, None)
    }

    /// Stable machine-readable code (the JSON envelope `code`).
    pub fn code(&self) -> &'static str {
        match self.kind {
            ErrorKind::Connection => "connection",
            ErrorKind::Timeout => "timeout",
            ErrorKind::NotFound => "not_found",
            ErrorKind::Data => "data",
            ErrorKind::Rejected => "rejected",
            ErrorKind::Unconfirmed => "unconfirmed",
            ErrorKind::Config => "config",
            ErrorKind::Guard => "guard",
            ErrorKind::Usage => "usage",
            ErrorKind::Other => "error",
        }
    }

    /// Process exit code per error kind.
    pub fn exit_code(&self) -> i32 {
        match self.kind {
            ErrorKind::Connection => 2,
            ErrorKind::Timeout => 6,
            ErrorKind::NotFound => 3,
            ErrorKind::Data => 4,
            ErrorKind::Rejected => 7,
            // Write-path only. Read timeouts stay [`ErrorKind::Timeout`] → 6.
            ErrorKind::Unconfirmed => 8,
            ErrorKind::Config => 5,
            ErrorKind::Guard => 64, // same bucket as usage: blocked before send
            ErrorKind::Usage => 64, // EX_USAGE
            ErrorKind::Other => 1,
        }
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.context {
            Some(ctx) => write!(f, "[{}] {} ({})", self.code(), self.message, ctx),
            None => write!(f, "[{}] {}", self.code(), self.message),
        }
    }
}

impl std::error::Error for AppError {}
