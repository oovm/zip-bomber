//! The `zb.report/v0` document shared by template stubs, successes, and errors.

use serde::{Deserialize, Serialize};

use crate::{ContainerKind, CrcStrategy, VariantKind};

/// Report schema id. There is one schema: nothing has been released, so the
/// stub and success paths share one id instead of a legacy/frozen pair.
pub const REPORT_SCHEMA: &str = "zb.report/v0";

/// Task-level budget consumption after generate / verify (Living `13` §5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BudgetUsage {
    /// Configured `max_decoded_bytes` ceiling.
    pub max_decoded_bytes: u64,
    /// Decoded bytes charged across member and codec self-checks.
    pub decoded_bytes_used: u64,
    /// Configured live-memory read ceiling.
    pub max_live_memory_bytes: u64,
    /// Read bytes charged against live-memory quota.
    pub read_bytes_used: u64,
    /// ZIP members decoded under the ledger.
    pub members_decoded: u64,
}

/// Compact solver evidence mirrored from `zb-solver::SolveEvidence`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SolverFacts {
    /// Matrix rank after elimination.
    pub rank: u32,
    /// Free bits under the free-variable = 0 policy.
    pub free_bits: u32,
    /// Equation count accepted into the matrix.
    pub equations_used: u32,
    /// Optional opaque digest of the frozen skeleton.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub problem_digest: Option<String>,
}

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
    /// Schema id ([`REPORT_SCHEMA`]).
    pub schema: String,
    /// Container.
    pub container: ContainerKind,
    /// Requested variant.
    pub variant: VariantKind,
    /// Outcome status.
    pub status: ReportStatus,
    /// Construction profile id when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
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
    /// Solver evidence when a fixed skeleton CRC / bit problem was solved.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub solver: Option<SolverFacts>,
    /// Structured Loopy facts when a two-node gapped cycle was constructed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loopy: Option<LoopyFacts>,
    /// Task budget ledger usage when a generate path tracked decode/read quotas.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget_usage: Option<BudgetUsage>,
}

impl GenerateReport {
    /// Build a `not_implemented` stub report for a variant.
    pub fn not_implemented(variant: VariantKind, crc: CrcStrategy, nodes: usize, edges: usize) -> Self {
        Self {
            schema: REPORT_SCHEMA.to_string(),
            container: ContainerKind::Zip,
            variant,
            status: ReportStatus::NotImplemented,
            profile: None,
            topology_nodes: nodes,
            topology_edges: edges,
            output_path: None,
            archive_bytes: None,
            declared_uncompressed: None,
            crc_strategy: crc.as_str().to_string(),
            warnings: vec![format!("variant `{variant}` generator is not implemented yet")],
            error_code: Some("ZB_NOT_IMPLEMENTED".to_string()),
            error_message: Some(format!("variant `{variant}` is recognized but not implemented")),
            solver: None,
            loopy: None,
            budget_usage: None,
        }
    }

    /// Successful emit report (`zb.report/v0` with the full fact set).
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
            schema: REPORT_SCHEMA.to_string(),
            container: ContainerKind::Zip,
            variant,
            status: ReportStatus::Ok,
            profile: None,
            topology_nodes: nodes,
            topology_edges: edges,
            output_path: None,
            archive_bytes: Some(archive_bytes),
            declared_uncompressed: Some(declared_uncompressed),
            crc_strategy: crc.as_str().to_string(),
            warnings,
            error_code: None,
            error_message: None,
            solver: None,
            loopy: None,
            budget_usage: None,
        }
    }

    /// Failed emit report carrying the structured error.
    pub fn failed(variant: VariantKind, crc: CrcStrategy, nodes: usize, edges: usize, err: &crate::ZbError) -> Self {
        Self {
            schema: REPORT_SCHEMA.to_string(),
            container: ContainerKind::Zip,
            variant,
            status: ReportStatus::Error,
            profile: None,
            topology_nodes: nodes,
            topology_edges: edges,
            output_path: None,
            archive_bytes: None,
            declared_uncompressed: None,
            crc_strategy: crc.as_str().to_string(),
            warnings: Vec::new(),
            error_code: Some(err.code.as_str().to_string()),
            error_message: Some(err.message.clone()),
            solver: None,
            loopy: None,
            budget_usage: None,
        }
    }

    /// Attach the active construction profile id.
    pub fn with_profile(mut self, profile: impl Into<String>) -> Self {
        self.profile = Some(profile.into());
        self
    }

    /// Attach solver evidence without mutating warnings.
    pub fn with_solver(mut self, facts: SolverFacts) -> Self {
        self.solver = Some(facts);
        self
    }

    /// Attach structured Loopy facts without mutating warnings.
    pub fn with_loopy(mut self, facts: LoopyFacts) -> Self {
        self.loopy = Some(facts);
        self
    }

    /// Attach task budget usage without mutating warnings.
    pub fn with_budget_usage(mut self, usage: BudgetUsage) -> Self {
        self.budget_usage = Some(usage);
        self
    }
}
