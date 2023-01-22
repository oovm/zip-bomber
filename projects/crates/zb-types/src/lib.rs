//! Shared contracts for zb recursive ZIP / zip-bomb generation.

#![deny(missing_docs)]

mod artifact;
mod capability;
mod container;
mod error;
mod graph;
mod outcome;
mod profile;
mod report;
mod request;
mod topology;
mod variant;
mod wire;

pub use artifact::{Artifact, ArtifactId, ArtifactSet, DecimalU64};
pub use capability::{CapabilityEntry, CapabilitySet, CapabilityStatus, TopologyClass};
pub use container::ContainerKind;
pub use error::{ErrorCategory, ErrorCode, ZbError, ZbResult};
pub use graph::{ArchiveGraphMember, ArchiveGraphNode, ArchiveGraphSpec};
pub use outcome::Outcome;
pub use profile::{
    zip_layout, GenerateLimits, ZipProfile, ZIP_LOOPY_TWO_NODE_GAPPED_V1, ZIP_PROFILE_V0,
};
pub use report::{
    GenerateReport, LoopyEdgeFact, LoopyFacts, ReportStatus, REPORT_SCHEMA_V1, REPORT_SCHEMA_V2,
};
pub use request::{
    CrcStrategy, FractalMode, FractalSpec, GenerateRequest, PayloadSpec, REQUEST_SCHEMA_V2,
};
pub use topology::{AlignmentPolicy, ArchiveEdge, EdgeRole, NodeId, TopologyGraph};
pub use variant::{VariantKind, ALL_VARIANTS};
pub use wire::{
    decode_base64, encode_base64, generate_request_from_json, generate_request_to_json,
    WireArchiveEdge, WireGenerateRequest, WirePayloadSpec, WireTopologyGraph,
};
