//! Independent archive verification (Acorn-backed later).

pub mod cycle;
pub mod zip;

pub use cycle::{
    cycle_edges_from_graph, minimum_byte_period, verify_artifact_set_for_request,
    verify_artifact_set_with_spec,
};
pub(crate) use zip::verify_zip_bytes_with_ledger;
pub use zip::{
    verify_artifact_set, verify_artifact_set_with_limits, verify_zip_bytes,
    verify_zip_bytes_with_limits, verify_zip_task_budget, AcornZipCapability,
    VerificationReport,
};
