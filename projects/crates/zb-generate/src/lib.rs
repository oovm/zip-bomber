//! Variant drivers for zb recursive ZIP generation.

#![deny(missing_docs)]

mod crc;
mod deflate;
mod emit;
mod layout;
mod templates;
mod validate;
mod variants;
mod verify;

use serde::{Deserialize, Serialize};
use zb_types::{
    ArtifactSet, CapabilitySet, GenerateReport, GenerateRequest, Outcome, VariantKind, ZbError,
    ZbResult, ALL_VARIANTS,
};

pub use crc::CrcPlan;
pub use deflate::{
    chain_repeats, encode_blocks, encode_fixed_stream, encode_stored_block, encode_stored_stream,
    expand_skeleton, validate_repeat, BitWriter, DeflateBlock, DeflatePlan, FixedOp, SkeletonOp,
};
pub use emit::{emit_stored_archive, StoredMember};
pub use layout::{archive_size, plan, resolve_profile, LayoutPlan, MemberLayout};
// `PlanSummary` / `plan_generate` are defined below (pipeline plan, not layout::plan).
/// Re-export solver surface so generate callers share one dependency path.
pub use zb_solver::{
    crc32, crc32_affine_columns, solve, solve_rows, u32_to_bits, Assignment, BitMatrix,
    EquationId, Gf2Solution, SolveOutcome, VariableId, CRC32_CHECK, CRC32_POLY,
};
pub use templates::{
    cox_core_program, interpret_lz_quine, is_self_reproducing, list_quine_templates, LzOp,
    QuineTemplateMeta,
};
pub use variants::ArtifactBundle;
pub use verify::{verify_artifact_set, verify_zip_bytes, AcornZipCapability, VerificationReport};

/// List supported variant ids in stable order.
pub fn list_variants() -> &'static [VariantKind] {
    &ALL_VARIANTS
}

/// Capability snapshot (`Bomb` nested tree is `implemented`, others declared).
pub fn capabilities() -> CapabilitySet {
    CapabilitySet::template_declared()
}

/// Feasibility plan (no archive bytes). Living `04` `plan` surface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanSummary {
    /// Whether generation is expected to succeed for this request/profile.
    pub feasible: bool,
    /// Requested variant id.
    pub variant: String,
    /// Resolved profile id.
    pub profile: String,
    /// Short human summary (stable English tokens for machines prefer `feasible`).
    pub summary: String,
    /// Non-fatal notes.
    pub warnings: Vec<String>,
}

/// Plan generation without emitting bytes.
pub fn plan_generate(request: GenerateRequest) -> Outcome<PlanSummary> {
    let variant = request.variant;
    let crc = request.crc_strategy;
    let nodes = request.topology.nodes.len();
    let edges = request.topology.edges.len();

    if let Err(err) = validate::normalize(&request) {
        let report = GenerateReport::failed(variant, crc, nodes, edges, &err);
        return Outcome::err(err, report);
    }
    let layout = match layout::plan(&request) {
        Ok(plan) => plan,
        Err(err) => {
            let report = GenerateReport::failed(variant, crc, nodes, edges, &err);
            return Outcome::err(err, report);
        }
    };

    match variant {
        VariantKind::Bomb => {
            let empty = request.topology.nodes.is_empty() && request.topology.edges.is_empty();
            let summary = if empty {
                format!(
                    "bomb feasible under `{}` via default depth nesting (empty topology)",
                    layout.profile_id
                )
            } else {
                format!(
                    "bomb feasible under `{}` via topology DAG (nodes={nodes} edges={edges})",
                    layout.profile_id
                )
            };
            let plan = PlanSummary {
                feasible: true,
                variant: variant.as_str().to_string(),
                profile: layout.profile_id,
                summary,
                warnings: Vec::new(),
            };
            let report = GenerateReport::ok(variant, crc, nodes, edges, 0, 0, Vec::new());
            Outcome::ok(plan, report)
        }
        VariantKind::Quine => {
            let registered = templates::list_quine_templates(&layout.profile_id).len();
            let err = ZbError::search_exhausted(format!(
                "quine plan: {registered} verified templates for `{}`",
                layout.profile_id
            ));
            let mut report = GenerateReport::failed(variant, crc, nodes, edges, &err);
            report.warnings.push(
                "quine requires a verified DEFLATE self-reference skeleton before emit".to_string(),
            );
            Outcome::err(err, report)
        }
        VariantKind::Loopy | VariantKind::Fractal => {
            let err = ZbError::not_implemented(format!(
                "plan for variant `{}` is not implemented yet",
                variant.as_str()
            ));
            let mut report = GenerateReport::not_implemented(variant, crc, nodes, edges);
            report.warnings.push(format!(
                "profile `{}` resolved but variant driver is not planned",
                layout.profile_id
            ));
            Outcome::err(err, report)
        }
    }
}

/// Generate an archive artifact for the request.
///
/// Template builds return an [`Outcome`] with `not_implemented` after topology validation.
pub fn generate(request: GenerateRequest) -> Outcome<ArtifactSet> {
    if let Err(err) = validate::normalize(&request) {
        let report = GenerateReport::not_implemented(
            request.variant,
            request.crc_strategy,
            request.topology.nodes.len(),
            request.topology.edges.len(),
        );
        return Outcome::err(err, report);
    }
    let _layout = match layout::plan(&request) {
        Ok(plan) => plan,
        Err(err) => {
            let report = GenerateReport::not_implemented(
                request.variant,
                request.crc_strategy,
                request.topology.nodes.len(),
                request.topology.edges.len(),
            );
            return Outcome::err(err, report);
        }
    };
    let _deflate = deflate::plan(&request);
    let _crc = crc::plan(&request);
    let variant = request.variant;
    let crc = request.crc_strategy;
    let nodes = request.topology.nodes.len();
    let edges = request.topology.edges.len();
    match variants::dispatch(request) {
        Ok(bundle) => {
            if bundle.report.status == zb_types::ReportStatus::NotImplemented {
                Outcome::err(
                    ZbError::not_implemented(
                        bundle
                            .report
                            .error_message
                            .clone()
                            .unwrap_or_else(|| "not implemented".to_string()),
                    ),
                    bundle.report,
                )
            } else if bundle.report.status == zb_types::ReportStatus::Error {
                let err = ZbError::new(
                    zb_types::ErrorCode::Internal,
                    bundle
                        .report
                        .error_message
                        .clone()
                        .unwrap_or_else(|| "generate failed".to_string()),
                );
                Outcome::err(err, bundle.report)
            } else {
                Outcome::ok(bundle.artifacts, bundle.report)
            }
        }
        Err(err) => {
            let report = GenerateReport::failed(variant, crc, nodes, edges, &err);
            Outcome::err(err, report)
        }
    }
}

/// Build a JSON report string.
pub fn report_json(report: &GenerateReport) -> ZbResult<String> {
    serde_json::to_string_pretty(report).map_err(|err| {
        ZbError::new(
            zb_types::ErrorCode::Internal,
            format!("failed to serialize report: {err}"),
        )
    })
}

/// Independent ZIP verification entry (returns capability gap until Acorn is wired).
pub fn verify(set: &ArtifactSet) -> ZbResult<VerificationReport> {
    verify_artifact_set(set)
}
