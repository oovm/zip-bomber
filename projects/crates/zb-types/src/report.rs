//! `zb.report/v1` (template) and `zb.report/v2` (design target) reports.

use serde::{Deserialize, Serialize};

use crate::{ContainerKind, CrcStrategy, VariantKind};

/// Legacy template report schema.
pub const REPORT_SCHEMA_V1: &str = "zb.report/v1";
/// Design-target report schema (fields freeze before algorithm gates).
pub const REPORT_SCHEMA_V2: &str = "zb.report/v2";

/// High-level generation status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportStatus {
    /// Bytes were produced successfully.
    Ok,
    /// Variant or algorithm path is not wired yet.
    NotImplemented,
    /// Hard failure.
    Error,
}

impl ReportStatus {
    /// Stable id.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::NotImplemented => "not_implemented",
            Self::Error => "error",
        }
    }
}

/// Structured generation report shared by CLI / N-API / WASM.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenerateReport {
    /// Schema id (`zb.report/v1` during template; migrate to v2 when frozen).
    pub schema: String,
    /// Container.
    pub container: ContainerKind,
    /// Requested variant.
    pub variant: VariantKind,
    /// Outcome status.
    pub status: ReportStatus,
    /// Node count in the topology.
    pub topology_nodes: usize,
    /// Edge count in the topology.
    pub topology_edges: usize,
    /// Output path when written to disk (optional, CLI only).
    pub output_path: Option<String>,
    /// Archive byte length when bytes exist.
    pub archive_bytes: Option<u64>,
    /// Declared uncompressed total when known.
    pub declared_uncompressed: Option<u64>,
    /// CRC strategy used or requested.
    pub crc_strategy: String,
    /// Non-fatal notes.
    pub warnings: Vec<String>,
    /// Stable error code when failed.
    pub error_code: Option<String>,
    /// Human error message when failed.
    pub error_message: Option<String>,
}

impl GenerateReport {
    /// Build a `not_implemented` stub report for a variant.
    pub fn not_implemented(
        variant: VariantKind,
        crc: CrcStrategy,
        nodes: usize,
        edges: usize,
    ) -> Self {
        Self {
            schema: REPORT_SCHEMA_V1.to_string(),
            container: ContainerKind::Zip,
            variant,
            status: ReportStatus::NotImplemented,
            topology_nodes: nodes,
            topology_edges: edges,
            output_path: None,
            archive_bytes: None,
            declared_uncompressed: None,
            crc_strategy: crc.as_str().to_string(),
            warnings: vec![format!(
                "variant `{variant}` generator is not implemented yet"
            )],
            error_code: Some("ZB_NOT_IMPLEMENTED".to_string()),
            error_message: Some(format!(
                "variant `{variant}` is recognized but not implemented"
            )),
        }
    }
}
