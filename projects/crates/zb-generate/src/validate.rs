//! Topology normalization against variant contracts.

use zb_types::{ErrorCode, GenerateRequest, VariantKind, ZbError, ZbResult};

/// Validate topology shape for the requested variant.
pub fn normalize(request: &GenerateRequest) -> ZbResult<()> {
    match request.variant {
        VariantKind::Quine => validate_quine(request),
        VariantKind::Loopy => validate_loopy(request),
        VariantKind::Bomb | VariantKind::Fractal => Ok(()),
    }
}

fn validate_quine(request: &GenerateRequest) -> ZbResult<()> {
    let graph = &request.topology;
    if graph.nodes.len() != 1 {
        return Err(ZbError::new(
            ErrorCode::InvalidTopology,
            "quine requires exactly one topology node",
        ));
    }
    if graph.edges.len() != 1 {
        return Err(ZbError::new(
            ErrorCode::InvalidTopology,
            "quine requires exactly one self-loop edge",
        ));
    }
    let edge = &graph.edges[0];
    if edge.from != edge.to || edge.from != graph.nodes[0] {
        return Err(ZbError::new(
            ErrorCode::InvalidTopology,
            "quine edge must be a self-loop on the sole node",
        ));
    }
    Ok(())
}

fn validate_loopy(request: &GenerateRequest) -> ZbResult<()> {
    let graph = &request.topology;
    if graph.nodes.len() < 2 {
        return Err(ZbError::new(
            ErrorCode::InvalidTopology,
            "loopy requires at least two topology nodes",
        ));
    }
    if graph.edges.len() < 2 {
        return Err(ZbError::new(
            ErrorCode::InvalidTopology,
            "loopy requires a directed cycle of length at least 2",
        ));
    }
    Ok(())
}
