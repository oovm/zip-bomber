//! Variant drivers for zb recursive ZIP generation.

#![deny(missing_docs)]

mod crc;
mod deflate;
mod emit;
mod gf2;
mod layout;
mod validate;
mod variants;
mod verify;

use zb_types::{
    ArtifactSet, CapabilitySet, GenerateReport, GenerateRequest, Outcome, VariantKind, ZbError,
    ZbResult, ALL_VARIANTS,
};

pub use crc::{
    crc32, crc32_affine_columns, u32_to_bits, CrcPlan, CRC32_CHECK, CRC32_POLY,
};
pub use deflate::{
    chain_repeats, encode_stored_block, expand_skeleton, validate_repeat, DeflatePlan, SkeletonOp,
};
pub use emit::{emit_stored_archive, StoredMember};
pub use gf2::{solve, solve_rows, BitMatrix, Gf2Solution};
pub use layout::{archive_size, plan, resolve_profile, LayoutPlan, MemberLayout};
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
