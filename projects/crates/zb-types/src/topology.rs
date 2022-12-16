//! Topology graph for nested / cyclic archives.

use serde::{Deserialize, Serialize};

/// Logical member or nested archive node.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub String);

impl NodeId {
    /// Create a node id from any string-like value.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

/// Directed edge meaning "extracting `from` yields `to`".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopologyEdge {
    /// Source archive node.
    pub from: NodeId,
    /// Nested archive or member produced by extraction.
    pub to: NodeId,
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
    /// Extraction edges.
    pub edges: Vec<TopologyEdge>,
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
    pub fn quine_self(name: impl Into<String>) -> Self {
        let id = NodeId::new(name);
        Self {
            nodes: vec![id.clone()],
            edges: vec![TopologyEdge {
                from: id.clone(),
                to: id,
            }],
            alignment: AlignmentPolicy::default(),
        }
    }

    /// Two-node cycle suitable for a loopy stub.
    pub fn loopy_pair(a: impl Into<String>, b: impl Into<String>) -> Self {
        let a = NodeId::new(a);
        let b = NodeId::new(b);
        Self {
            nodes: vec![a.clone(), b.clone()],
            edges: vec![
                TopologyEdge {
                    from: a.clone(),
                    to: b.clone(),
                },
                TopologyEdge { from: b, to: a },
            ],
            alignment: AlignmentPolicy::default(),
        }
    }
}
