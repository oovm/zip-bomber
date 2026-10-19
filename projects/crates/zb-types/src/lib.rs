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
pub use capability::{CapabilityEntry, CapabilitySet, CapabilityStatus, TopologyClass};
pub use container::ContainerKind;
pub use decimal::{format_decimal_u64, parse_decimal_u64};
pub use error::{ErrorCategory, ErrorCode, ZbError, ZbResult};
pub use graph::{ArchiveGraphMember, ArchiveGraphNode, ArchiveGraphSpec};
pub use outcome::Outcome;
pub use profile::{GenerateLimits, ZIP_LOOPY_TWO_NODE_GAPPED_V0, ZIP_PROFILE_V0, ZipProfile, zip_layout};
pub use report::{BudgetUsage, GenerateReport, LoopyEdgeFact, LoopyFacts, REPORT_SCHEMA, ReportStatus, SolverFacts};
pub use request::{CrcStrategy, FractalMode, FractalSpec, GenerateRequest, PayloadSpec, REQUEST_SCHEMA};
pub use topology::{AlignmentPolicy, ArchiveEdge, EdgeRole, NodeId, TopologyGraph};
pub use variant::{ALL_VARIANTS, VariantKind};
pub use verification::{CycleVerifyFacts, VerificationSpec, VerifyEdgeSpec};
pub use wire::{
    WireArchiveEdge, WireArtifact, WireArtifactSet, WireGenerateLimits, WireGenerateRequest, WirePayloadSpec,
    WireTopologyGraph, artifact_set_from_json, artifact_set_to_json, decode_base64, encode_base64, generate_request_from_json,
    generate_request_to_json, verification_spec_from_json, verification_spec_to_json,
};
