//! Independent archive verification (Acorn-backed later).

pub mod zip;

pub use zip::{verify_artifact_set, verify_zip_bytes, AcornZipCapability, VerificationReport};
