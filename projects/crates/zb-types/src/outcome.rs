//! Structured success / failure outcome shared by Node and WASM.

use serde::{Deserialize, Serialize};

use crate::{GenerateReport, ZbError};

/// API outcome carrying a report on both branches.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Outcome<T> {
    /// Successful value plus report.
    Ok {
        /// Success marker for wire consumers.
        ok: bool,
        /// Payload.
        value: T,
        /// Structured report.
        report: GenerateReport,
    },
    /// Failed call plus report.
    Err {
        /// Failure marker for wire consumers.
        ok: bool,
        /// Structured error.
        error: ZbError,
        /// Structured report.
        report: GenerateReport,
    },
}

impl<T> Outcome<T> {
    /// Build a success outcome.
    pub fn ok(value: T, report: GenerateReport) -> Self {
        Self::Ok {
            ok: true,
            value,
            report,
        }
    }

    /// Build a failure outcome.
    pub fn err(error: ZbError, report: GenerateReport) -> Self {
        Self::Err {
            ok: false,
            error,
            report,
        }
    }

    /// Whether the outcome is success.
    pub fn is_ok(&self) -> bool {
        matches!(self, Self::Ok { .. })
    }
}
