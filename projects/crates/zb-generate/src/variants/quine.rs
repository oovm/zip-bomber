//! Strict self-referential ZIP quine (stub).

use zb_types::{GenerateRequest, ZbResult};

use super::{stub_artifact, Artifact};

/// Generate a quine archive. Template returns `not_implemented` report.
pub fn generate(request: GenerateRequest) -> ZbResult<Artifact> {
    Ok(stub_artifact(request))
}
