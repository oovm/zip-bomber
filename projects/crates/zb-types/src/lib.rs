//! Shared contracts for zb recursive ZIP / zip-bomb generation.

#![deny(missing_docs)]

mod error;
mod report;
mod request;
mod topology;
mod variant;

pub use error::{ErrorCode, ZbError, ZbResult};
pub use report::{GenerateReport, ReportStatus, REPORT_SCHEMA};
pub use request::{CrcStrategy, GenerateRequest, PayloadSpec};
pub use topology::{AlignmentPolicy, NodeId, TopologyEdge, TopologyGraph};
pub use variant::{VariantKind, ALL_VARIANTS};
