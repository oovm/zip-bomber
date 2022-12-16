//! Classic nested / overlapping expansion bomb (stub).

use zb_types::{GenerateRequest, ZbResult};

use super::{stub_bundle, ArtifactBundle};

/// Generate a bomb archive. Template returns `not_implemented` report.
pub fn generate(request: GenerateRequest) -> ZbResult<ArtifactBundle> {
    Ok(stub_bundle(request))
}
