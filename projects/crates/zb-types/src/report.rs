//! `zb.report/v1` (template) and `zb.report/v2` (design target) reports.

use serde::{Deserialize, Serialize};

use crate::{ContainerKind, CrcStrategy, VariantKind};

/// Legacy template report schema.
pub const REPORT_SCHEMA_V1: &str = "zb.report/v1";
/// Design-target report schema (fields freeze before algorithm gates).
pub const REPORT_SCHEMA_V2: &str = "zb.report/v2";

/// One directed cycle edge after independent inflate / CRC checks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoopyEdgeFact {
    /// Source state index in the request graph.
    pub from: usize,
    /// Target state index.
    pub to: usize,
    /// Recursive member path on the source archive.
    pub path: String,
    /// Compressed stream length for the recursive member.
    pub compressed_len: u64,
    /// Target archive length.
    pub target_len: u64,
    /// Target archive CRC-32.
    pub target_crc: u32,
    /// Inflate(source recursive member) equals target bytes.
    pub bytes_equal: bool,
}

/// Structured Loopy / recursive two-node facts (Living 11). Not stuffed into warnings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoopyFacts {
    /// Requested cycle length.
    pub requested_nodes: usize,
    /// Minimum byte period of the produced cyclic word.
    pub byte_period: usize,
    /// Construction profile id.
    pub profile: String,
    /// Ordinary (non-archive) member counts per cycle state in root order.
    pub ordinary_members: Vec<usize>,
    /// Quote window budget (`ordinary || early-footer || recursive-LFH`).
    pub quote_window: u64,
    /// Measured ordinary local widths before padding, per cycle state.
    pub ordinary_widths: Vec<u64>,
    /// Shared CRC system rank when solved.
    pub crc_rank: u32,
    /// Free bits after rank (constraint columns minus rank).
    pub crc_free_bits: u32,
    /// Per-edge verification facts in cycle order.
    pub edges: Vec<LoopyEdgeFact>,
}

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
    /// Structured Loopy facts when a two-node gapped cycle was constructed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loopy: Option<LoopyFacts>,
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
            loopy: None,
        }
    }

    /// Successful emit report.
    pub fn ok(
        variant: VariantKind,
        crc: CrcStrategy,
        nodes: usize,
        edges: usize,
        archive_bytes: u64,
        declared_uncompressed: u64,
        warnings: Vec<String>,
    ) -> Self {
        Self {
            schema: REPORT_SCHEMA_V1.to_string(),
            container: ContainerKind::Zip,
            variant,
            status: ReportStatus::Ok,
            topology_nodes: nodes,
            topology_edges: edges,
            output_path: None,
            archive_bytes: Some(archive_bytes),
            declared_uncompressed: Some(declared_uncompressed),
            crc_strategy: crc.as_str().to_string(),
            warnings,
            error_code: None,
            error_message: None,
            loopy: None,
        }
    }

    /// Failed emit report carrying the structured error.
    pub fn failed(
        variant: VariantKind,
        crc: CrcStrategy,
        nodes: usize,
        edges: usize,
        err: &crate::ZbError,
    ) -> Self {
        Self {
            schema: REPORT_SCHEMA_V1.to_string(),
            container: ContainerKind::Zip,
            variant,
            status: ReportStatus::Error,
            topology_nodes: nodes,
            topology_edges: edges,
            output_path: None,
            archive_bytes: None,
            declared_uncompressed: None,
            crc_strategy: crc.as_str().to_string(),
            warnings: Vec::new(),
            error_code: Some(err.code.as_str().to_string()),
            error_message: Some(err.message.clone()),
            loopy: None,
        }
    }

    /// Attach structured Loopy facts without mutating warnings.
    pub fn with_loopy(mut self, facts: LoopyFacts) -> Self {
        self.loopy = Some(facts);
        self
    }
}
