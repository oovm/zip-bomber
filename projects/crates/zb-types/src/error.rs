//! Stable error codes for zb.

use std::fmt;

/// Stable machine-readable error code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    /// Variant id is unknown.
    UnknownVariant,
    /// Topology violates the variant contract.
    InvalidTopology,
    /// Algorithm path is not wired yet.
    NotImplemented,
    /// Internal invariant broke.
    Internal,
}

impl ErrorCode {
    /// Stable string form.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::UnknownVariant => "ZB_UNKNOWN_VARIANT",
            Self::InvalidTopology => "ZB_INVALID_TOPOLOGY",
            Self::NotImplemented => "ZB_NOT_IMPLEMENTED",
            Self::Internal => "ZB_INTERNAL",
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// zb fallible result error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZbError {
    /// Stable code.
    pub code: ErrorCode,
    /// Human message.
    pub message: String,
}

impl ZbError {
    /// Construct an error.
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    /// Not-implemented helper.
    pub fn not_implemented(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::NotImplemented, message)
    }
}

impl fmt::Display for ZbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for ZbError {}

/// Convenient result alias.
pub type ZbResult<T> = Result<T, ZbError>;
