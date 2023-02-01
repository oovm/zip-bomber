//! Shared contracts for zb recursive ZIP / zip-bomb generation.

#![deny(missing_docs)]

mod artifact;
mod capability;
mod container;
mod decimal;
mod error;
mod graph;
mod outcome;
mod profile;
mod report;
mod request;
mod topology;
mod variant;
mod verification;
mod wire;

pub use artifact::{Artifact, ArtifactId, ArtifactSet, DecimalU64};
pub use decimal::{format_decimal_u64, parse_decimal_u64};
pub use capability::{CapabilityEntry, CapabilitySet, CapabilityStatus, TopologyClass};
pub use container::ContainerKind;
pub use error::{ErrorCategory, ErrorCode, ZbError, ZbResult};
pub use graph::{ArchiveGraphMember, ArchiveGraphNode, ArchiveGraphSpec};
pub use outcome::Outcome;
pub use profile::{
    zip_layout, GenerateLimits, ZipProfile, ZIP_LOOPY_TWO_NODE_GAPPED_V1, ZIP_PROFILE_V0,
};
pub use report::{
    GenerateReport, LoopyEdgeFact, LoopyFacts, ReportStatus, SolverFacts, REPORT_SCHEMA_V1,
    REPORT_SCHEMA_V2,
};
pub use request::{
    CrcStrategy, FractalMode, FractalSpec, GenerateRequest, PayloadSpec, REQUEST_SCHEMA_V2,
};
pub use topology::{AlignmentPolicy, ArchiveEdge, EdgeRole, NodeId, TopologyGraph};
pub use variant::{VariantKind, ALL_VARIANTS};
pub use verification::{CycleVerifyFacts, VerificationSpec, VerifyEdgeSpec};
pub use wire::{
    artifact_set_from_json, artifact_set_to_json, decode_base64, encode_base64,
    generate_request_from_json, generate_request_to_json, verification_spec_from_json,
    verification_spec_to_json, WireArchiveEdge, WireArtifact, WireArtifactSet, WireGenerateLimits,
    WireGenerateRequest, WirePayloadSpec, WireTopologyGraph,
};
