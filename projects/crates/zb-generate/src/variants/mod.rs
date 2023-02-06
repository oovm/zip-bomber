//! Per-variant generate drivers.

mod bomb;
mod fractal;
mod loopy;
pub(crate) mod quine;

use zb_solver::CancelToken;
use zb_types::{ArtifactSet, GenerateReport, GenerateRequest, VariantKind, ZbResult};

use crate::PlanBudgetEstimate;

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
    dispatch_with_cancel(request, &CancelToken::new())
}

/// [`dispatch`] with cooperative cancel for Quine CRC solve steps.
pub fn dispatch_with_cancel(
    request: GenerateRequest,
    cancel: &CancelToken,
) -> ZbResult<ArtifactBundle> {
    match request.variant {
        VariantKind::Bomb => bomb::generate(request),
        VariantKind::Quine => quine::generate_with_cancel(request, cancel),
        VariantKind::Loopy => loopy::generate(request),
        VariantKind::Fractal => fractal::generate(request),
    }
}

/// Plan-time budget estimate for finite / topology bomb requests.
pub(crate) fn plan_bomb_budget(request: &GenerateRequest) -> ZbResult<PlanBudgetEstimate> {
    bomb::plan_budget(request)
}

/// Plan-time budget estimate for quine requests.
pub(crate) fn plan_quine_budget(request: &GenerateRequest) -> ZbResult<PlanBudgetEstimate> {
    quine::plan_materialization_budget(request)
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
