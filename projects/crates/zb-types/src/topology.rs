//! Topology graph for nested / cyclic archives.

use serde::{Deserialize, Serialize};

/// Logical planning identity for an archive node.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub String);

impl NodeId {
    /// Create a node id from any string-like value.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

/// Role of a member edge inside a production or nest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeRole {
    /// Default nested archive member.
    NestedArchive,
    /// Quine self-reference member.
    SelfReference,
    /// Ordinary payload (non-recursive).
    Payload,
    /// Named fractal production role.
    FractalRole,
}

impl EdgeRole {
    /// Stable id.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NestedArchive => "nested_archive",
            Self::SelfReference => "self_reference",
            Self::Payload => "payload",
            Self::FractalRole => "fractal_role",
        }
    }
}

/// Directed archive edge: extracting `from` / `member_path` yields `to`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchiveEdge {
    /// Optional stable edge id.
    pub id: Option<String>,
    /// Source archive node.
    pub from: NodeId,
    /// Member path inside `from` (POSIX-style, `/` separators).
    pub member_path: String,
    /// Nested archive or member produced by extraction.
    pub to: NodeId,
    /// Semantic role of this member.
    pub role: EdgeRole,
}

/// Extra Field / header-footer alignment policy (required for loopy).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlignmentPolicy {
    /// Force equal local-header sizes across ring members.
    pub equal_headers: bool,
    /// Force equal central-directory / footer sizes across ring members.
    pub equal_footers: bool,
    /// Soft budget for Extra Field padding in bytes.
    pub extra_budget: u32,
}

impl Default for AlignmentPolicy {
    fn default() -> Self {
        Self {
            equal_headers: true,
            equal_footers: true,
            extra_budget: 32_763,
        }
    }
}

/// Include graph for a generation request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopologyGraph {
    /// Logical nodes.
    pub nodes: Vec<NodeId>,
    /// Extraction edges with member paths.
    pub edges: Vec<ArchiveEdge>,
    /// Header / footer alignment knobs.
    pub alignment: AlignmentPolicy,
}

impl TopologyGraph {
    /// Empty graph with default alignment.
    pub fn empty() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            alignment: AlignmentPolicy::default(),
        }
    }

    /// Single-node self-loop suitable for a quine stub.
    pub fn quine_self(name: impl Into<String>, member_path: impl Into<String>) -> Self {
        let id = NodeId::new(name);
        Self {
            nodes: vec![id.clone()],
            edges: vec![ArchiveEdge {
                id: None,
                from: id.clone(),
                member_path: member_path.into(),
                to: id,
                role: EdgeRole::SelfReference,
            }],
            alignment: AlignmentPolicy::default(),
        }
    }

    /// Two-node cycle suitable for a loopy stub.
    pub fn loopy_pair(
        a: impl Into<String>,
        b: impl Into<String>,
        a_member: impl Into<String>,
        b_member: impl Into<String>,
    ) -> Self {
        let a = NodeId::new(a);
        let b = NodeId::new(b);
        Self {
            nodes: vec![a.clone(), b.clone()],
            edges: vec![
                ArchiveEdge {
                    id: None,
                    from: a.clone(),
                    member_path: a_member.into(),
                    to: b.clone(),
                    role: EdgeRole::NestedArchive,
                },
                ArchiveEdge {
                    id: None,
                    from: b,
                    member_path: b_member.into(),
                    to: a,
                    role: EdgeRole::NestedArchive,
                },
            ],
            alignment: AlignmentPolicy::default(),
        }
    }
}
