//! Per-variant generate drivers.

mod bomb;
mod fractal;
mod loopy;
mod quine;

use zb_types::{GenerateReport, GenerateRequest, VariantKind, ZbResult};

/// Generated archive bytes plus report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Artifact {
    /// Archive bytes when implemented (empty for stubs).
    pub bytes: Vec<u8>,
    /// Structured report.
    pub report: GenerateReport,
}

/// Dispatch to the variant driver.
pub fn dispatch(request: GenerateRequest) -> ZbResult<Artifact> {
    match request.variant {
        VariantKind::Bomb => bomb::generate(request),
        VariantKind::Quine => quine::generate(request),
        VariantKind::Loopy => loopy::generate(request),
        VariantKind::Fractal => fractal::generate(request),
    }
}

pub(super) fn stub_artifact(request: GenerateRequest) -> Artifact {
    let report = GenerateReport::not_implemented(
        request.variant,
        request.crc_strategy,
        request.topology.nodes.len(),
        request.topology.edges.len(),
    );
    Artifact {
        bytes: Vec::new(),
        report,
    }
}
