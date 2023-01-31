//! Independent archive verification (Acorn-backed later).

pub mod cycle;
pub mod zip;

pub use cycle::{
    cycle_edges_from_graph, minimum_byte_period, verify_artifact_set_for_request,
    verify_artifact_set_with_spec,
};
pub use zip::{verify_artifact_set, verify_zip_bytes, AcornZipCapability, VerificationReport};
