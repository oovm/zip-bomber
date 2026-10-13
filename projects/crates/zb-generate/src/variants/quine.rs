//! Strict self-referential ZIP quine.

use zb_types::{GenerateRequest, ZbError, ZbResult, ZIP_PROFILE_V0};

use crate::templates::{list_quine_templates, QuineTemplateMeta};

use super::ArtifactBundle;

/// Generate a quine archive by enumerating verified templates.
///
/// Returns [`ZbError::search_exhausted`] when the registry has no candidate for
/// the request profile. Topology must already pass [`crate::validate`] quine
/// rules (single self-loop).
pub fn generate(request: GenerateRequest) -> ZbResult<ArtifactBundle> {
    let profile = if request.profile.is_empty() {
        ZIP_PROFILE_V0
    } else {
        request.profile.as_str()
    };
    let candidates = list_quine_templates(profile);
    if candidates.is_empty() {
        return Err(ZbError::search_exhausted(format!(
            "no verified quine templates registered for profile `{profile}` (searched 0 candidates)"
        )));
    }

    let mut attempts = Vec::new();
    for meta in &candidates {
        match try_template(&request, meta) {
            Ok(bundle) => return Ok(bundle),
            Err(err) => attempts.push(format!("{}: {}", meta.id, err.message)),
        }
    }

    Err(ZbError::search_exhausted(format!(
        "quine template search exhausted for profile `{profile}` after {} candidates ({})",
        candidates.len(),
        attempts.join("; ")
    )))
}

fn try_template(
    _request: &GenerateRequest,
    meta: &QuineTemplateMeta,
) -> ZbResult<ArtifactBundle> {
    // Emit / GF(2) solve / Acorn verify land with the first registered skeleton.
    Err(ZbError::not_implemented(format!(
        "quine template `{}` has metadata but no emit path yet",
        meta.id
    )))
}
