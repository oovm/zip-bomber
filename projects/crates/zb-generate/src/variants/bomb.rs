//! Classic nested / overlapping expansion bomb (stub).

use zb_types::{GenerateRequest, ZbResult};

use super::{stub_artifact, Artifact};

/// Generate a bomb archive. Template returns `not_implemented` report.
pub fn generate(request: GenerateRequest) -> ZbResult<Artifact> {
    Ok(stub_artifact(request))
}
