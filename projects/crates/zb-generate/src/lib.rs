//! Variant drivers for zb recursive ZIP generation.

#![deny(missing_docs)]

mod crc;
mod deflate;
mod layout;
mod validate;
mod variants;

use zb_types::{GenerateReport, GenerateRequest, VariantKind, ZbError, ZbResult, ALL_VARIANTS};

pub use variants::Artifact;

/// List supported variant ids in stable order.
pub fn list_variants() -> &'static [VariantKind] {
    &ALL_VARIANTS
}

/// Generate an archive artifact for the request.
///
/// Template builds return [`ZbError`] with `NotImplemented` after topology validation.
pub fn generate(request: GenerateRequest) -> ZbResult<Artifact> {
    validate::normalize(&request)?;
    let _layout = layout::plan(&request);
    let _deflate = deflate::plan(&request);
    let _crc = crc::plan(&request);
    variants::dispatch(request)
}

/// Build a JSON `zb.report/v1` string for a successful or stub result.
pub fn report_json(report: &GenerateReport) -> ZbResult<String> {
    serde_json::to_string_pretty(report).map_err(|err| {
        ZbError::new(
            zb_types::ErrorCode::Internal,
            format!("failed to serialize report: {err}"),
        )
    })
}
