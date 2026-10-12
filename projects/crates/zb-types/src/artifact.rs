//! Artifact identities and sets.

use serde::{Deserialize, Serialize};

use crate::ContainerKind;

/// Decimal u64 carried as a decimal string on the wire.
pub type DecimalU64 = String;

/// Byte-identity of an emitted archive (distinct from planning [`crate::NodeId`]).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ArtifactId(pub String);

impl ArtifactId {
    /// Create an artifact id.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

/// One emitted archive blob.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Artifact {
    /// Stable artifact id.
    pub id: ArtifactId,
    /// Container kind.
    pub container: ContainerKind,
    /// Archive bytes.
    pub bytes: Vec<u8>,
    /// Decimal byte length.
    pub byte_length: DecimalU64,
    /// Content digest (algorithm name + hex), empty until emit.
    pub digest: String,
}

/// Set of artifacts returned by generate / verify.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactSet {
    /// Root artifact id.
    pub root: ArtifactId,
    /// Deterministic-order artifacts.
    pub artifacts: Vec<Artifact>,
    /// Planning node id → artifact id map entries as `node=artifact` pairs.
    pub logical_node_map: Vec<String>,
}

impl ArtifactSet {
    /// Empty stub set (template).
    pub fn empty_stub(root: impl Into<String>) -> Self {
        let root = ArtifactId::new(root);
        Self {
            root,
            artifacts: Vec::new(),
            logical_node_map: Vec::new(),
        }
    }
}
