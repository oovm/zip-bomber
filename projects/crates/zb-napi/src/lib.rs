//! Node-API export surface for zb.

#![deny(missing_docs)]

use napi::bindgen_prelude::*;
use napi_derive::napi;
use zb_generate::{
    capabilities, generate, list_variants, plan_generate, report_json, verify_zip_bytes,
};
use zb_types::{GenerateRequest, Outcome, ReportStatus, VariantKind};

/// N-API generate response.
#[napi(object)]
pub struct GenerateResponse {
    /// Process-style exit code. Zero only when status is `ok`.
    pub exit_code: u32,
    /// Whether the outcome succeeded.
    pub ok: bool,
    /// Root archive bytes when present.
    pub bytes: Option<Buffer>,
    /// Serialized report JSON.
    pub report_json: String,
    /// Stable error category when failed.
    pub error_category: Option<String>,
    /// Stable error code when failed.
    pub error_code: Option<String>,
    /// Human error message when failed.
    pub error_message: Option<String>,
}

/// N-API capability row.
#[napi(object)]
pub struct CapabilityRow {
    /// Container id.
    pub container: String,
    /// Variant id.
    pub variant: String,
    /// Topology class id.
    pub topology_class: String,
    /// Profile id.
    pub profile: String,
    /// Capability status.
    pub status: String,
}

/// List supported variant ids.
#[napi]
pub fn list_variants_napi() -> Vec<String> {
    list_variants()
        .iter()
        .map(|v| v.as_str().to_string())
        .collect()
}

/// Capability matrix snapshot.
#[napi]
pub fn capabilities_napi() -> Vec<CapabilityRow> {
    capabilities()
        .entries
        .into_iter()
        .map(|e| CapabilityRow {
            container: e.container.as_str().to_string(),
            variant: e.variant.as_str().to_string(),
            topology_class: match e.topology_class {
                zb_types::TopologyClass::NestedTree => "nested_tree".to_string(),
                zb_types::TopologyClass::SelfLoop => "self_loop".to_string(),
                zb_types::TopologyClass::DirectedCycle => "directed_cycle".to_string(),
                zb_types::TopologyClass::FractalFinite => "fractal_finite".to_string(),
                zb_types::TopologyClass::FractalRecursive => "fractal_recursive".to_string(),
            },
            profile: e.profile,
            status: match e.status {
                zb_types::CapabilityStatus::Declared => "declared".to_string(),
                zb_types::CapabilityStatus::Implemented => "implemented".to_string(),
                zb_types::CapabilityStatus::Verified => "verified".to_string(),
            },
        })
        .collect()
}

/// N-API plan response (no archive bytes).
#[napi(object)]
pub struct PlanResponse {
    /// Process-style exit code. Zero when planning succeeded.
    pub exit_code: u32,
    /// Whether the outcome succeeded.
    pub ok: bool,
    /// Whether generation is expected to succeed.
    pub feasible: Option<bool>,
    /// Short plan summary when present.
    pub summary: Option<String>,
    /// Resolved profile id when present.
    pub profile: Option<String>,
    /// Serialized generate report JSON.
    pub report_json: String,
    /// Stable error category when failed.
    pub error_category: Option<String>,
    /// Stable error code when failed.
    pub error_code: Option<String>,
    /// Human error message when failed.
    pub error_message: Option<String>,
}

/// N-API verify response for raw ZIP bytes.
#[napi(object)]
pub struct VerifyResponse {
    /// Process-style exit code. Zero when verification succeeded.
    pub exit_code: u32,
    /// Whether verification succeeded.
    pub ok: bool,
    /// Serialized verification report JSON when available.
    pub report_json: String,
    /// Stable error category when failed.
    pub error_category: Option<String>,
    /// Stable error code when failed.
    pub error_code: Option<String>,
    /// Human error message when failed.
    pub error_message: Option<String>,
}

/// Plan a bare variant without emitting bytes.
#[napi]
pub fn plan_variant(variant: String) -> Result<PlanResponse> {
    let kind = VariantKind::parse(&variant)
        .ok_or_else(|| Error::from_reason(format!("unknown variant `{variant}`")))?;
    let request = GenerateRequest::bare(kind);
    match plan_generate(request) {
        Outcome::Ok { value, report, .. } => {
            let report_json =
                report_json(&report).map_err(|err| Error::from_reason(err.to_string()))?;
            Ok(PlanResponse {
                exit_code: 0,
                ok: true,
                feasible: Some(value.feasible),
                summary: Some(value.summary),
                profile: Some(value.profile),
                report_json,
                error_category: None,
                error_code: None,
                error_message: None,
            })
        }
        Outcome::Err { error, report, .. } => {
            let report_json =
                report_json(&report).map_err(|err| Error::from_reason(err.to_string()))?;
            let exit_code = match report.status {
                ReportStatus::Ok => 0,
                ReportStatus::NotImplemented => 2,
                ReportStatus::Error => 1,
            };
            Ok(PlanResponse {
                exit_code,
                ok: false,
                feasible: Some(false),
                summary: None,
                profile: None,
                report_json,
                error_category: Some(error.category.as_str().to_string()),
                error_code: Some(error.code.as_str().to_string()),
                error_message: Some(error.message),
            })
        }
    }
}

/// Independently verify raw ZIP archive bytes through Acorn.
#[napi]
pub fn verify_zip_bytes_napi(bytes: Buffer) -> Result<VerifyResponse> {
    match verify_zip_bytes(bytes.as_ref()) {
        Ok(report) => {
            let report_json = serde_json::to_string_pretty(&report)
                .map_err(|err| Error::from_reason(err.to_string()))?;
            Ok(VerifyResponse {
                exit_code: if report.ok { 0 } else { 1 },
                ok: report.ok,
                report_json,
                error_category: None,
                error_code: None,
                error_message: None,
            })
        }
        Err(error) => Ok(VerifyResponse {
            exit_code: match error.code {
                zb_types::ErrorCode::NotImplemented
                | zb_types::ErrorCode::VerificationCapabilityMissing => 2,
                _ => 1,
            },
            ok: false,
            report_json: "{}".to_string(),
            error_category: Some(error.category.as_str().to_string()),
            error_code: Some(error.code.as_str().to_string()),
            error_message: Some(error.message),
        }),
    }
}

/// Generate an archive for a lowercase variant id using a bare default topology.
#[napi]
pub fn generate_variant(variant: String) -> Result<GenerateResponse> {
    let kind = VariantKind::parse(&variant)
        .ok_or_else(|| Error::from_reason(format!("unknown variant `{variant}`")))?;
    let request = GenerateRequest::bare(kind);
    match generate(request) {
        Outcome::Ok { value, report, .. } => {
            let report_json =
                report_json(&report).map_err(|err| Error::from_reason(err.to_string()))?;
            let exit_code = match report.status {
                ReportStatus::Ok => 0,
                ReportStatus::NotImplemented => 2,
                ReportStatus::Error => 1,
            };
            let bytes = value
                .artifacts
                .iter()
                .find(|a| a.id == value.root)
                .or_else(|| value.artifacts.last())
                .filter(|a| !a.bytes.is_empty())
                .map(|a| Buffer::from(a.bytes.clone()));
            Ok(GenerateResponse {
                exit_code,
                ok: true,
                bytes,
                report_json,
                error_category: None,
                error_code: None,
                error_message: None,
            })
        }
        Outcome::Err { error, report, .. } => {
            let report_json =
                report_json(&report).map_err(|err| Error::from_reason(err.to_string()))?;
            let exit_code = match report.status {
                ReportStatus::Ok => 0,
                ReportStatus::NotImplemented => 2,
                ReportStatus::Error => 1,
            };
            Ok(GenerateResponse {
                exit_code,
                ok: false,
                bytes: None,
                report_json,
                error_category: Some(error.category.as_str().to_string()),
                error_code: Some(error.code.as_str().to_string()),
                error_message: Some(error.message),
            })
        }
    }
}
