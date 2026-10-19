//! Generate request and payload slots (`zb.request/v0`).

use serde::{Deserialize, Serialize};

use crate::{ContainerKind, GenerateLimits, TopologyGraph, VariantKind, ZIP_PROFILE_V0, zip_layout};

/// Request schema id.
pub const REQUEST_SCHEMA: &str = "zb.request/v0";

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

/// Optional embedded member (README, fixture bytes, directory entry, …).
///
/// `path` is the archive-relative POSIX path (`archivePath` on the TS surface).
/// A trailing `/` with empty `bytes` denotes an explicit directory entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PayloadSpec {
    /// Archive member path (UTF-8). Serde also accepts `archivePath`.
    #[serde(alias = "archivePath", alias = "archive_path")]
    pub path: String,
    /// Frozen input snapshot bytes to embed.
    #[serde(default)]
    pub bytes: Vec<u8>,
}

/// Fractal expansion mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FractalMode {
    /// Bounded depth expansion.
    Finite,
    /// Recursive SCC / replacement grammar.
    Recursive,
}

impl FractalMode {
    /// Stable id.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Finite => "finite",
            Self::Recursive => "recursive",
        }
    }
}

/// Minimal fractal grammar stub (full productions land with algorithm gates).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FractalSpec {
    /// Expansion mode.
    pub mode: FractalMode,
    /// Required for [`FractalMode::Finite`].
    pub depth: Option<u32>,
    /// Root symbol name.
    pub root_symbol: String,
}

/// Full generation request (`zb.request/v0` shape).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenerateRequest {
    /// Always [`REQUEST_SCHEMA`].
    pub schema: String,
    /// Archive container.
    pub container: ContainerKind,
    /// Variant driver to run.
    pub variant: VariantKind,
    /// Ordered state graph used by staged builders.
    #[serde(default)]
    pub graph: Option<crate::ArchiveGraphSpec>,
    /// Include / cycle topology.
    pub topology: TopologyGraph,
    /// Optional payload slots.
    pub payloads: Vec<PayloadSpec>,
    /// Optional fractal grammar when `variant == Fractal`.
    pub fractal: Option<FractalSpec>,
    /// Construction profile id (must match a known [`crate::ZipProfile`]).
    pub profile: String,
    /// Execution budgets.
    pub limits: GenerateLimits,
    /// CRC resolution strategy.
    pub crc_strategy: CrcStrategy,
    /// Soft Extra Field / header budget in bytes.
    pub extra_budget: u32,
}

impl GenerateRequest {
    /// Minimal request for a variant with a default topology.
    pub fn bare(variant: VariantKind) -> Self {
        let topology = match variant {
            VariantKind::Quine => TopologyGraph::quine_self("a.zip", "a/a.zip"),
            VariantKind::Loopy => TopologyGraph::loopy_pair("a.zip", "b.zip", "b.zip", "a.zip"),
            VariantKind::Bomb | VariantKind::Fractal => TopologyGraph::empty(),
        };
        let fractal = matches!(variant, VariantKind::Fractal).then(|| FractalSpec {
            mode: FractalMode::Finite,
            depth: Some(1),
            root_symbol: "S".to_string(),
        });
        Self {
            schema: REQUEST_SCHEMA.to_string(),
            container: ContainerKind::Zip,
            variant,
            graph: None,
            topology,
            payloads: Vec::new(),
            fractal,
            profile: ZIP_PROFILE_V0.to_string(),
            limits: GenerateLimits::small_sample(),
            crc_strategy: CrcStrategy::default(),
            extra_budget: zip_layout::EXTRA_BUDGET_DEFAULT,
        }
    }
}
