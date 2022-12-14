//! `zb.report/v1` shaped generation report.

use serde::{Deserialize, Serialize};

use crate::{CrcStrategy, VariantKind};

/// Report schema id.
pub const REPORT_SCHEMA: &str = "zb.report/v1";

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
    /// Always [`REPORT_SCHEMA`].
    pub schema: String,
    /// Requested variant.
    pub variant: VariantKind,
    /// Outcome status.
    pub status: ReportStatus,
    /// Node count in the topology.
    pub topology_nodes: usize,
    /// Edge count in the topology.
    pub topology_edges: usize,
    /// Output path when written to disk (optional).
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
            schema: REPORT_SCHEMA.to_string(),
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
