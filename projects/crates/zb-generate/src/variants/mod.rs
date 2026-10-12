//! Per-variant generate drivers.

mod bomb;
mod fractal;
mod loopy;
mod quine;

use zb_types::{ArtifactSet, GenerateReport, GenerateRequest, VariantKind, ZbResult};

/// Generated archive set plus report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactBundle {
    /// Artifact set (empty for stubs).
    pub artifacts: ArtifactSet,
    /// Structured report.
    pub report: GenerateReport,
}

/// Dispatch to the variant driver.
pub fn dispatch(request: GenerateRequest) -> ZbResult<ArtifactBundle> {
    match request.variant {
        VariantKind::Bomb => bomb::generate(request),
        VariantKind::Quine => quine::generate(request),
        VariantKind::Loopy => loopy::generate(request),
        VariantKind::Fractal => fractal::generate(request),
    }
}

pub(super) fn stub_bundle(request: GenerateRequest) -> ArtifactBundle {
    let report = GenerateReport::not_implemented(
        request.variant,
        request.crc_strategy,
        request.topology.nodes.len(),
        request.topology.edges.len(),
    );
    ArtifactBundle {
        artifacts: ArtifactSet::empty_stub("root"),
        report,
    }
}
