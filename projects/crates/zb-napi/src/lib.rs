//! Node-API export surface for zb.

#![deny(missing_docs)]

use napi::bindgen_prelude::*;
use napi_derive::napi;
use zb_generate::{generate, list_variants, report_json};
use zb_types::{GenerateRequest, ReportStatus, VariantKind};

/// N-API generate response.
#[napi(object)]
pub struct GenerateResponse {
    /// Process-style exit code. Zero only when status is `ok`.
    pub exit_code: u32,
    /// Archive bytes when implemented.
    pub bytes: Option<Buffer>,
    /// Serialized `zb.report/v1` JSON.
    pub report_json: String,
}

/// List supported variant ids.
#[napi]
pub fn list_variants_napi() -> Vec<String> {
    list_variants()
        .iter()
        .map(|v| v.as_str().to_string())
        .collect()
}

/// Generate an archive for a lowercase variant id using a bare default topology.
#[napi]
pub fn generate_variant(variant: String) -> Result<GenerateResponse> {
    let kind = VariantKind::parse(&variant)
        .ok_or_else(|| Error::from_reason(format!("unknown variant `{variant}`")))?;
    let request = GenerateRequest::bare(kind);
    let artifact = generate(request).map_err(|err| Error::from_reason(err.to_string()))?;
    let report_json =
        report_json(&artifact.report).map_err(|err| Error::from_reason(err.to_string()))?;
    let exit_code = match artifact.report.status {
        ReportStatus::Ok => 0,
        ReportStatus::NotImplemented => 2,
        ReportStatus::Error => 1,
    };
    let bytes = if artifact.bytes.is_empty() {
        None
    } else {
        Some(Buffer::from(artifact.bytes))
    };
    Ok(GenerateResponse {
        exit_code,
        bytes,
        report_json,
    })
}
