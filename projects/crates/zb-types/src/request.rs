//! Generate request and payload slots.

use serde::{Deserialize, Serialize};

use crate::{TopologyGraph, VariantKind};

/// How CRC32 fixed points will be resolved (algorithm module later).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CrcStrategy {
    /// Patch bytes that reset CRC state mid-stream (template default).
    ResetBytes,
    /// Solve coupled CRCs over GF(2) (future).
    GaussianElimination,
}

impl Default for CrcStrategy {
    fn default() -> Self {
        Self::ResetBytes
    }
}

impl CrcStrategy {
    /// Stable id for reports.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ResetBytes => "reset_bytes",
            Self::GaussianElimination => "gaussian_elimination",
        }
    }
}

/// Optional embedded member (README, fixture bytes, …).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PayloadSpec {
    /// Archive member path.
    pub path: String,
    /// Raw bytes to embed when the generator supports payloads.
    pub bytes: Vec<u8>,
}

/// Full generation request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenerateRequest {
    /// Variant driver to run.
    pub variant: VariantKind,
    /// Include / cycle topology.
    pub topology: TopologyGraph,
    /// Optional payload slots.
    pub payloads: Vec<PayloadSpec>,
    /// CRC resolution strategy.
    pub crc_strategy: CrcStrategy,
    /// Soft Extra Field / header budget in bytes.
    pub extra_budget: u32,
}

impl GenerateRequest {
    /// Minimal request for a variant with an empty topology.
    pub fn bare(variant: VariantKind) -> Self {
        let topology = match variant {
            VariantKind::Quine => TopologyGraph::quine_self("a.zip"),
            VariantKind::Loopy => TopologyGraph::loopy_pair("a.zip", "b.zip"),
            VariantKind::Bomb | VariantKind::Fractal => TopologyGraph::empty(),
        };
        Self {
            variant,
            topology,
            payloads: Vec::new(),
            crc_strategy: CrcStrategy::default(),
            extra_budget: 32_763,
        }
    }
}
