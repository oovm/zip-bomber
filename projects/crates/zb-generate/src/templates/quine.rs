//! Quine template registry.
//!
//! Complete prefix/core/suffix skeletons accepted by bounded independent
//! decoding and Acorn verification. Cross-backend capability remains separate.

use zb_types::ZIP_PROFILE_V0;

/// Metadata for one quine layout / DEFLATE skeleton candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuineTemplateMeta {
    /// Stable template id (does not include version numbers).
    pub id: &'static str,
    /// ZIP profile id this template was verified against.
    pub profile: &'static str,
    /// Human-readable notes for reports.
    pub notes: &'static str,
}

/// List quine templates registered for `profile`.
///
/// Unsupported profiles have no registered candidates.
pub fn list_quine_templates(profile: &str) -> Vec<QuineTemplateMeta> {
    QUINE_TEMPLATES.iter().copied().filter(|t| t.profile == profile).collect()
}

const QUINE_TEMPLATES: &[QuineTemplateMeta] = &[QuineTemplateMeta {
    id: "quine.stored-fixed.prefix-suffix",
    profile: ZIP_PROFILE_V0,
    notes: "Cox-style quoted prefix/core/suffix with explicit terminal blocks",
}];

// Keep the profile constant referenced so the empty table stays profile-aware.
const _: &str = ZIP_PROFILE_V0;
