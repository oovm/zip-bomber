//! Shared contracts for zb recursive ZIP / zip-bomb generation.

#![deny(missing_docs)]

mod artifact;
mod capability;
mod container;
mod error;
mod outcome;
mod profile;
mod report;
mod request;
mod topology;
mod variant;

pub use artifact::{Artifact, ArtifactId, ArtifactSet, DecimalU64};
pub use capability::{
    CapabilityEntry, CapabilitySet, CapabilityStatus, TopologyClass,
};
pub use container::ContainerKind;
pub use error::{ErrorCategory, ErrorCode, ZbError, ZbResult};
pub use outcome::Outcome;
pub use profile::{zip_layout, GenerateLimits, ZipProfile, ZIP_PROFILE_V0};
pub use report::{GenerateReport, ReportStatus, REPORT_SCHEMA_V1, REPORT_SCHEMA_V2};
pub use request::{
    CrcStrategy, FractalMode, FractalSpec, GenerateRequest, PayloadSpec, REQUEST_SCHEMA_V2,
};
pub use topology::{
    AlignmentPolicy, ArchiveEdge, EdgeRole, NodeId, TopologyGraph,
};
pub use variant::{VariantKind, ALL_VARIANTS};
