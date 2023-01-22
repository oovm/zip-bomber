//! Capability matrix entries.

use serde::{Deserialize, Serialize};

use crate::{
    ContainerKind, VariantKind, ZIP_LOOPY_TWO_NODE_GAPPED_V1, ZIP_PROFILE_V0,
};

/// Coarse topology class for capability registration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TopologyClass {
    /// Nested tree without cycles.
    NestedTree,
    /// Single-node self loop.
    SelfLoop,
    /// Directed cycle length ≥ 2.
    DirectedCycle,
    /// Finite replacement grammar.
    FractalFinite,
    /// Recursive SCC fractal.
    FractalRecursive,
}

/// Declared vs implemented vs independently verified.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityStatus {
    /// Present in the public contract.
    Declared,
    /// Code path exists but not fully verified.
    Implemented,
    /// Passed generate + Acorn + independent decode + cross-backend checks.
    Verified,
}

/// One capability row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityEntry {
    /// Container.
    pub container: ContainerKind,
    /// Variant.
    pub variant: VariantKind,
    /// Topology class.
    pub topology_class: TopologyClass,
    /// Profile id (versioned ZIP profile name).
    pub profile: String,
    /// Status.
    pub status: CapabilityStatus,
}

/// Capability snapshot returned by `capabilities()`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilitySet {
    /// Rows.
    pub entries: Vec<CapabilityEntry>,
}

impl CapabilitySet {
    /// Current capability snapshot keyed by variant × topology × profile.
    pub fn template_declared() -> Self {
        let rows = [
            (
                VariantKind::Bomb,
                TopologyClass::NestedTree,
                ZIP_PROFILE_V0,
                CapabilityStatus::Implemented,
            ),
            (
                VariantKind::Quine,
                TopologyClass::SelfLoop,
                ZIP_PROFILE_V0,
                CapabilityStatus::Verified,
            ),
            (
                VariantKind::Loopy,
                TopologyClass::DirectedCycle,
                ZIP_LOOPY_TWO_NODE_GAPPED_V1,
                CapabilityStatus::Verified,
            ),
            (
                VariantKind::Fractal,
                TopologyClass::FractalFinite,
                ZIP_PROFILE_V0,
                CapabilityStatus::Implemented,
            ),
            (
                VariantKind::Fractal,
                TopologyClass::FractalRecursive,
                ZIP_LOOPY_TWO_NODE_GAPPED_V1,
                CapabilityStatus::Implemented,
            ),
        ];
        Self {
            entries: rows
                .into_iter()
                .map(|(variant, topology_class, profile, status)| CapabilityEntry {
                    container: ContainerKind::Zip,
                    variant,
                    topology_class,
                    profile: profile.to_string(),
                    status,
                })
                .collect(),
        }
    }
}
