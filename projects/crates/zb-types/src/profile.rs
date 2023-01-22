//! Versioned ZIP construction profiles.

use serde::{Deserialize, Serialize};

/// Profile id for the first-wave ZIP stub.
pub const ZIP_PROFILE_V0: &str = "zip.profile/v0-stub";

/// Explicit opt-in profile for two-node Loopy / Fractal recursive construction
/// with a duplicated early directory gap (Living `11`).
pub const ZIP_LOOPY_TWO_NODE_GAPPED_V1: &str = "zip.loopy-two-node.gapped/v1";

/// Fixed sizes from APPNOTE / Living `05` (not estimates).
pub mod zip_layout {
    /// Local file header fixed prefix (before name / extra).
    pub const LFH_FIXED: u32 = 30;
    /// Central directory entry fixed prefix including 4-byte signature.
    pub const CD_ENTRY_FIXED: u32 = 46;
    /// End of central directory minimum size.
    pub const EOCD_MIN: u32 = 22;
    /// Maximum DEFLATE match length.
    pub const DEFLATE_LEN_MAX: u32 = 258;
    /// Minimum DEFLATE match length.
    pub const DEFLATE_LEN_MIN: u32 = 3;
    /// Maximum DEFLATE distance (32 KiB window).
    pub const DEFLATE_DIST_MAX: u32 = 32_768;
    /// Soft Extra Field budget referenced by classic templates.
    pub const EXTRA_BUDGET_DEFAULT: u32 = 32_763;
}

/// Named construction profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ZipProfile {
    /// Stable profile id.
    pub id: String,
    /// Single-volume archives only.
    pub single_volume: bool,
    /// Encryption disallowed.
    pub allow_encryption: bool,
    /// Data descriptors disallowed in v0.
    pub allow_data_descriptor: bool,
    /// Trailing junk after EOCD disallowed.
    pub allow_trailing_data: bool,
    /// Overlapping members disallowed unless a future overlap profile.
    pub allow_overlapping_members: bool,
    /// UTF-8 general purpose bit 11 required for names.
    pub utf8_names: bool,
    /// Stored payloads for ordinary members.
    pub store_payloads: bool,
    /// Raw DEFLATE for recursive archive members.
    pub deflate_nested_archives: bool,
    /// ZIP64 handled as a separate future profile.
    pub zip64: bool,
    /// Allow the template-declared early central-directory / EOCD gap before the
    /// recursive local header. Strict no-gap profiles keep this false.
    pub allow_early_directory_gap: bool,
}

impl ZipProfile {
    /// First-wave stub profile (Living `05` §3).
    pub fn v0_stub() -> Self {
        Self {
            id: ZIP_PROFILE_V0.to_string(),
            single_volume: true,
            allow_encryption: false,
            allow_data_descriptor: false,
            allow_trailing_data: false,
            allow_overlapping_members: false,
            utf8_names: true,
            store_payloads: true,
            deflate_nested_archives: true,
            zip64: false,
            allow_early_directory_gap: false,
        }
    }

    /// Two-node Loopy gapped profile (Living `11` §5).
    pub fn loopy_two_node_gapped_v1() -> Self {
        Self {
            id: ZIP_LOOPY_TWO_NODE_GAPPED_V1.to_string(),
            single_volume: true,
            allow_encryption: false,
            allow_data_descriptor: false,
            allow_trailing_data: false,
            allow_overlapping_members: false,
            utf8_names: true,
            store_payloads: true,
            deflate_nested_archives: true,
            zip64: false,
            allow_early_directory_gap: true,
        }
    }
}

/// Execution budgets from Living `04` (finite non-negative; no Infinity).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct GenerateLimits {
    /// Max topology nodes.
    pub max_nodes: u32,
    /// Max topology edges.
    pub max_edges: u32,
    /// Max members per archive.
    pub max_members: u32,
    /// Max total input payload bytes.
    pub max_input_bytes: u64,
    /// Max single artifact bytes.
    pub max_artifact_bytes: u64,
    /// Max sum of artifact bytes.
    pub max_total_artifact_bytes: u64,
    /// Max decode budget for verification unfold.
    pub max_decoded_bytes: u64,
    /// Max nesting / fractal depth.
    pub max_depth: u32,
    /// Max outer template candidates.
    pub max_template_candidates: u32,
    /// Max GF(2) constraint bits.
    pub max_constraint_bits: u64,
    /// Max solver word operations.
    pub max_solver_operations: u64,
    /// Max live working memory.
    pub max_live_memory_bytes: u64,
}

impl Default for GenerateLimits {
    fn default() -> Self {
        Self::small_sample()
    }
}

impl GenerateLimits {
    /// Shared convenience defaults (Living `04` small-sample profile).
    pub fn small_sample() -> Self {
        Self {
            max_nodes: 64,
            max_edges: 256,
            max_members: 64,
            max_input_bytes: 8 * 1024 * 1024,
            max_artifact_bytes: 8 * 1024 * 1024,
            max_total_artifact_bytes: 32 * 1024 * 1024,
            max_decoded_bytes: 64 * 1024 * 1024,
            max_depth: 8,
            max_template_candidates: 256,
            max_constraint_bits: 1 << 20,
            max_solver_operations: 1 << 26,
            max_live_memory_bytes: 64 * 1024 * 1024,
        }
    }
}
