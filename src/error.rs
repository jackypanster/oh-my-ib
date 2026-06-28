//! Structured application error + JSON error envelope. Fast-fail: every error
//! carries a concrete message and (where known) the operation/target context.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// Could not reach / handshake with the IB Gateway.
    Connection,
    /// A requested entity (contract, account) does not exist.
    NotFound,
    /// The gateway accepted the request but returned an error/notice.
    Data,
    /// Bad local config or flag combination.
    Config,
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
    pub fn not_found(message: impl Into<String>, context: impl Into<String>) -> Self {
        Self::new(ErrorKind::NotFound, message, Some(context.into()))
    }
    pub fn data(message: impl Into<String>, context: impl Into<String>) -> Self {
        Self::new(ErrorKind::Data, message, Some(context.into()))
    }
    pub fn config(message: impl Into<String>, context: impl Into<String>) -> Self {
        Self::new(ErrorKind::Config, message, Some(context.into()))
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
            ErrorKind::NotFound => "not_found",
            ErrorKind::Data => "data",
            ErrorKind::Config => "config",
            ErrorKind::Usage => "usage",
            ErrorKind::Other => "error",
        }
    }

    /// Process exit code per error kind.
    pub fn exit_code(&self) -> i32 {
        match self.kind {
            ErrorKind::Connection => 2,
            ErrorKind::NotFound => 3,
            ErrorKind::Data => 4,
            ErrorKind::Config => 5,
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
