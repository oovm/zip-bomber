//! Capability matrix entries.

use serde::{Deserialize, Serialize};

use crate::{ContainerKind, VariantKind};

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
    /// Template snapshot: all ZIP variants declared, none implemented.
    pub fn template_declared() -> Self {
        let profile = "zip.profile/v0-stub".to_string();
        let rows = [
            (VariantKind::Bomb, TopologyClass::NestedTree),
            (VariantKind::Quine, TopologyClass::SelfLoop),
            (VariantKind::Loopy, TopologyClass::DirectedCycle),
            (VariantKind::Fractal, TopologyClass::FractalFinite),
            (VariantKind::Fractal, TopologyClass::FractalRecursive),
        ];
        Self {
            entries: rows
                .into_iter()
                .map(|(variant, topology_class)| CapabilityEntry {
                    container: ContainerKind::Zip,
                    variant,
                    topology_class,
                    profile: profile.clone(),
                    status: CapabilityStatus::Declared,
                })
                .collect(),
        }
    }
}
