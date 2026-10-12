//! Variant drivers for zb recursive ZIP generation.

#![deny(missing_docs)]

mod crc;
mod deflate;
mod layout;
mod validate;
mod variants;

use zb_types::{
    ArtifactSet, CapabilitySet, GenerateReport, GenerateRequest, Outcome, VariantKind, ZbError,
    ZbResult, ALL_VARIANTS,
};

pub use variants::ArtifactBundle;

/// List supported variant ids in stable order.
pub fn list_variants() -> &'static [VariantKind] {
    &ALL_VARIANTS
}

/// Capability snapshot (all ZIP variants declared, none implemented).
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
            } else {
                Outcome::ok(bundle.artifacts, bundle.report)
            }
        }
        Err(err) => {
            let report = GenerateReport::not_implemented(variant, crc, nodes, edges);
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
