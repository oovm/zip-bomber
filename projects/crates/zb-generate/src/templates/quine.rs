//! Quine template registry.
//!
//! No verified DEFLATE self-reference skeleton is registered yet. Drivers still
//! go through this table so adding the first Cox-style candidate is a localized
//! change (Living `10` §10).

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
/// Currently empty for all profiles. The first verified skeleton lands here.
pub fn list_quine_templates(profile: &str) -> Vec<QuineTemplateMeta> {
    QUINE_TEMPLATES
        .iter()
        .copied()
        .filter(|t| t.profile == profile)
        .collect()
}

const QUINE_TEMPLATES: &[QuineTemplateMeta] = &[
    // Intentionally empty until a DEFLATE self-reference skeleton is verified.
    // Example shape once ready:
    // QuineTemplateMeta {
    //     id: "quine.stored-fixed.cox-core",
    //     profile: ZIP_PROFILE_V0,
    //     notes: "Cox L4/R4 translated to stored+fixed blocks",
    // },
];

// Keep the profile constant referenced so the empty table stays profile-aware.
const _: &str = ZIP_PROFILE_V0;
