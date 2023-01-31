//! Independent verify contracts (Living `04` / `07` / `11`).

use serde::{Deserialize, Serialize};

use crate::LoopyEdgeFact;

/// One directed archive edge to re-check after generate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifyEdgeSpec {
    /// Source artifact id.
    pub from: String,
    /// Recursive member path on the source archive.
    pub member_path: String,
    /// Target artifact id whose bytes must equal the inflated member.
    pub to: String,
}

/// Optional cycle / period checks layered on top of ZIP structure verify.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct VerificationSpec {
    /// Directed edges to inflate and compare.
    #[serde(default)]
    pub edges: Vec<VerifyEdgeSpec>,
    /// When set, the cyclic byte word period must equal this value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_byte_period: Option<usize>,
}

/// Structured cycle evidence returned from independent verify.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CycleVerifyFacts {
    /// Minimum positive period of the cyclic artifact byte word.
    pub byte_period: usize,
    /// Requested period when the caller supplied one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_byte_period: Option<usize>,
    /// Per-edge inflate / CRC / equality facts in request order.
    pub edges: Vec<LoopyEdgeFact>,
}
