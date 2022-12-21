//! Stable error codes and categories for zb.

use std::fmt;

use serde::{Deserialize, Serialize};

/// Stable public error category (Living `04`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCategory {
    /// Request failed schema / path / limits checks.
    InvalidRequest,
    /// Container not supported.
    UnsupportedContainer,
    /// Variant not supported.
    UnsupportedVariant,
    /// Topology class not supported for the profile.
    UnsupportedTopology,
    /// Profile id unknown or mismatched.
    UnsupportedProfile,
    /// Native / WASM backend unavailable.
    BackendUnavailable,
    /// Binding ABI / schema mismatch.
    BindingMismatch,
    /// Budget exhausted.
    ResourceLimited,
    /// Finite template search ended without a solution.
    SearchExhausted,
    /// Fixed template equations were unsatisfiable.
    ConstraintUnsatisfied,
    /// Loopy period does not match the contract.
    PeriodMismatch,
    /// Independent verification failed.
    VerificationFailed,
    /// Acorn / decoder facts required by the profile are not available yet.
    VerificationCapabilityMissing,
    /// Cooperative cancel.
    Cancelled,
    /// Internal invariant broke.
    InternalError,
    /// Algorithm path is not wired yet (template).
    NotImplemented,
}

impl ErrorCategory {
    /// Stable string form.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InvalidRequest => "invalid_request",
            Self::UnsupportedContainer => "unsupported_container",
            Self::UnsupportedVariant => "unsupported_variant",
            Self::UnsupportedTopology => "unsupported_topology",
            Self::UnsupportedProfile => "unsupported_profile",
            Self::BackendUnavailable => "backend_unavailable",
            Self::BindingMismatch => "binding_mismatch",
            Self::ResourceLimited => "resource_limited",
            Self::SearchExhausted => "search_exhausted",
            Self::ConstraintUnsatisfied => "constraint_unsatisfied",
            Self::PeriodMismatch => "period_mismatch",
            Self::VerificationFailed => "verification_failed",
            Self::VerificationCapabilityMissing => "verification_capability_missing",
            Self::Cancelled => "cancelled",
            Self::InternalError => "internal_error",
            Self::NotImplemented => "not_implemented",
        }
    }
}

impl fmt::Display for ErrorCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Stable machine-readable error code (legacy template codes retained).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorCode {
    /// Variant id is unknown.
    UnknownVariant,
    /// Topology violates the variant contract.
    InvalidTopology,
    /// Profile id is unknown or mismatched.
    UnsupportedProfile,
    /// Verifier lacks required Acorn / decoder facts.
    VerificationCapabilityMissing,
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
            Self::UnsupportedProfile => "ZB_UNSUPPORTED_PROFILE",
            Self::VerificationCapabilityMissing => "ZB_VERIFICATION_CAPABILITY_MISSING",
            Self::NotImplemented => "ZB_NOT_IMPLEMENTED",
            Self::Internal => "ZB_INTERNAL",
        }
    }

    /// Map to public category.
    pub const fn category(self) -> ErrorCategory {
        match self {
            Self::UnknownVariant => ErrorCategory::UnsupportedVariant,
            Self::InvalidTopology => ErrorCategory::InvalidRequest,
            Self::UnsupportedProfile => ErrorCategory::UnsupportedProfile,
            Self::VerificationCapabilityMissing => ErrorCategory::VerificationCapabilityMissing,
            Self::NotImplemented => ErrorCategory::NotImplemented,
            Self::Internal => ErrorCategory::InternalError,
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// zb fallible result error.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ZbError {
    /// Public category.
    pub category: ErrorCategory,
    /// Stable code.
    pub code: ErrorCode,
    /// Human message.
    pub message: String,
}

impl ZbError {
    /// Construct an error from a legacy code.
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            category: code.category(),
            code,
            message: message.into(),
        }
    }

    /// Not-implemented helper.
    pub fn not_implemented(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::NotImplemented, message)
    }

    /// Missing verifier capability helper (Living `06`).
    pub fn verification_capability_missing(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::VerificationCapabilityMissing, message)
    }
}

impl fmt::Display for ZbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({}): {}", self.category, self.code, self.message)
    }
}

impl std::error::Error for ZbError {}

/// Convenient result alias.
pub type ZbResult<T> = Result<T, ZbError>;
