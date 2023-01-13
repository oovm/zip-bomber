//! Ordered archive-state graph shared by recursive and finite evaluation.
use crate::FractalMode;
use serde::{Deserialize, Serialize};

/// Normalized machine-indexed graph. Human callers use owner-bound TS handles.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchiveGraphSpec {
    /// Entry state index.
    pub root: usize,
    /// Evaluation semantics.
    pub mode: FractalMode,
    /// Decimal finite edge depth, absent for recursive evaluation.
    #[serde(default)]
    pub depth: Option<String>,
    /// Ordered states.
    pub nodes: Vec<ArchiveGraphNode>,
}
/// One archive state with ordered members and an optional terminal definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchiveGraphNode {
    /// Normal state members.
    pub members: Vec<ArchiveGraphMember>,
    /// Complete ordinary-member definition at finite depth zero.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminal: Option<Vec<ArchiveGraphMember>>,
}
/// An ordered payload or archive reference. Payload bytes use base64 on JSON wire.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ArchiveGraphMember {
    /// Ordinary file or empty directory.
    Payload {
        /// Relative UTF-8 path.
        path: String,
        /// Base64 input snapshot.
        bytes: String,
    },
    /// Archive member whose decoded bytes equal the target state.
    Archive {
        /// Relative member path.
        path: String,
        /// Target machine index.
        target: usize,
        /// Optional semantic role, defaulting to the member path.
        #[serde(default)]
        role: Option<String>,
    },
}
impl ArchiveGraphMember {
    /// Member path independent of kind.
    pub fn path(&self) -> &str {
        match self {
            Self::Payload { path, .. } | Self::Archive { path, .. } => path,
        }
    }
}
