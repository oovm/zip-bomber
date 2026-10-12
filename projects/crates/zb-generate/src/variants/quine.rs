//! Strict self-referential ZIP quine (stub).

use zb_types::{GenerateRequest, ZbResult};

use super::{stub_bundle, ArtifactBundle};

/// Generate a quine archive. Template returns `not_implemented` report.
pub fn generate(request: GenerateRequest) -> ZbResult<ArtifactBundle> {
    Ok(stub_bundle(request))
}
