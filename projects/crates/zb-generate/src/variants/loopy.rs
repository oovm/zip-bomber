//! Directed-cycle loopy ZIP (stub).

use zb_types::{GenerateRequest, ZbResult};

use super::{ArtifactBundle, stub_bundle};

/// Generate a loopy archive. Template returns `not_implemented` report.
pub fn generate(request: GenerateRequest) -> ZbResult<ArtifactBundle> {
    Ok(stub_bundle(request))
}
