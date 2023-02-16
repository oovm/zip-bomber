//! Node-API export surface for zb.

#![deny(missing_docs)]

use napi::bindgen_prelude::*;
use napi_derive::napi;
use zb_generate::{
    CancelToken, capabilities, generate_with_cancel, list_variants, plan_generate, report_json,
    verify_artifact_set_for_request, verify_artifact_set_with_spec, verify_zip_bytes,
};
use zb_types::{
    GenerateRequest, Outcome, ReportStatus, VariantKind, artifact_set_from_json, generate_request_from_json,
    verification_spec_from_json,
};

/// N-API generate response.
#[napi(object)]
pub struct ArtifactResponse {
    /// Stable logical artifact id.
    pub id: String,
    /// Archive bytes.
    pub bytes: Buffer,
    /// Decimal byte length.
    pub byte_length: String,
    /// Content digest.
    pub digest: String,
}

/// N-API generate response.
#[napi(object)]
pub struct GenerateResponse {
    /// Process-style exit code. Zero only when status is `ok`.
    pub exit_code: u32,
    /// Whether the outcome succeeded.
    pub ok: bool,
    /// Actual root artifact identity.
    pub root: Option<String>,
    /// Root archive bytes when present.
    pub bytes: Option<Buffer>,
    /// All emitted logical artifacts.
    pub artifacts: Vec<ArtifactResponse>,
    /// Logical node mapping entries.
    pub logical_node_map: Vec<String>,
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
    list_variants().iter().map(|v| v.as_str().to_string()).collect()
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
    /// Serialized `PlanBudgetEstimate` JSON when the plan produced one.
    pub budget_estimate_json: Option<String>,
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

fn plan_response(request: GenerateRequest) -> Result<PlanResponse> {
    match plan_generate(request) {
        Outcome::Ok { value, report, .. } => {
            let report_json = report_json(&report).map_err(|err| Error::from_reason(err.to_string()))?;
            let budget_estimate_json = match value.budget_estimate.as_ref() {
                Some(estimate) => Some(serde_json::to_string(estimate).map_err(|err| Error::from_reason(err.to_string()))?),
                None => None,
            };
            Ok(PlanResponse {
                exit_code: 0,
                ok: true,
                feasible: Some(value.feasible),
                summary: Some(value.summary),
                profile: Some(value.profile),
                report_json,
                budget_estimate_json,
                error_category: None,
                error_code: None,
                error_message: None,
            })
        }
        Outcome::Err { error, report, .. } => {
            let report_json = report_json(&report).map_err(|err| Error::from_reason(err.to_string()))?;
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
                budget_estimate_json: None,
                error_category: Some(error.category.as_str().to_string()),
                error_code: Some(error.code.as_str().to_string()),
                error_message: Some(error.message),
            })
        }
    }
}

fn generate_response(request: GenerateRequest) -> Result<GenerateResponse> {
    generate_response_with_cancel(request, &CancelToken::new())
}

fn generate_response_with_cancel(request: GenerateRequest, cancel: &CancelToken) -> Result<GenerateResponse> {
    match generate_with_cancel(request, cancel) {
        Outcome::Ok { value, report, .. } => {
            let report_json = report_json(&report).map_err(|err| Error::from_reason(err.to_string()))?;
            let exit_code = match report.status {
                ReportStatus::Ok => 0,
                ReportStatus::NotImplemented => 2,
                ReportStatus::Error => 1,
            };
            let artifacts = value
                .artifacts
                .iter()
                .map(|a| ArtifactResponse {
                    id: a.id.0.clone(),
                    bytes: Buffer::from(a.bytes.clone()),
                    byte_length: a.byte_length.clone(),
                    digest: a.digest.clone(),
                })
                .collect();
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
                root: Some(value.root.0.clone()),
                bytes,
                artifacts,
                logical_node_map: value.logical_node_map,
                report_json,
                error_category: None,
                error_code: None,
                error_message: None,
            })
        }
        Outcome::Err { error, report, .. } => {
            let report_json = report_json(&report).map_err(|err| Error::from_reason(err.to_string()))?;
            let exit_code = match report.status {
                ReportStatus::Ok => 0,
                ReportStatus::NotImplemented => 2,
                ReportStatus::Error => 1,
            };
            Ok(GenerateResponse {
                exit_code,
                ok: false,
                root: None,
                bytes: None,
                artifacts: Vec::new(),
                logical_node_map: Vec::new(),
                report_json,
                error_category: Some(error.category.as_str().to_string()),
                error_code: Some(error.code.as_str().to_string()),
                error_message: Some(error.message),
            })
        }
    }
}

/// Plan a bare variant without emitting bytes.
#[napi]
pub fn plan_variant(variant: String) -> Result<PlanResponse> {
    let kind = VariantKind::parse(&variant).ok_or_else(|| Error::from_reason(format!("unknown variant `{variant}`")))?;
    plan_response(GenerateRequest::bare(kind))
}

/// Plan from a full `zb.request/v0` JSON wire document.
#[napi]
pub fn plan_request_json(request_json: String) -> Result<PlanResponse> {
    let request = generate_request_from_json(&request_json).map_err(|err| Error::from_reason(err.to_string()))?;
    plan_response(request)
}

fn verify_response(result: zb_types::ZbResult<zb_generate::VerificationReport>) -> Result<VerifyResponse> {
    match result {
        Ok(report) => {
            let report_json = serde_json::to_string_pretty(&report).map_err(|err| Error::from_reason(err.to_string()))?;
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
                zb_types::ErrorCode::NotImplemented | zb_types::ErrorCode::VerificationCapabilityMissing => 2,
                zb_types::ErrorCode::PeriodMismatch => 1,
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

/// Independently verify raw ZIP archive bytes through Acorn.
#[napi]
pub fn verify_zip_bytes_napi(bytes: Buffer) -> Result<VerifyResponse> {
    verify_response(verify_zip_bytes(bytes.as_ref()))
}

/// Verify a full artifact set JSON, optionally with a cycle [`VerificationSpec`] JSON.
#[napi]
pub fn verify_artifact_set_json(set_json: String, spec_json: Option<String>) -> Result<VerifyResponse> {
    let set = artifact_set_from_json(&set_json).map_err(|err| Error::from_reason(err.to_string()))?;
    match spec_json {
        Some(spec) if !spec.trim().is_empty() => {
            let spec = verification_spec_from_json(&spec).map_err(|err| Error::from_reason(err.to_string()))?;
            verify_response(verify_artifact_set_with_spec(&set, &spec))
        }
        _ => verify_response(zb_generate::verify_artifact_set(&set)),
    }
}

/// Verify an artifact set using edges / period inferred from a `zb.request/v0` JSON.
#[napi]
pub fn verify_artifact_set_for_request_json(set_json: String, request_json: String) -> Result<VerifyResponse> {
    let set = artifact_set_from_json(&set_json).map_err(|err| Error::from_reason(err.to_string()))?;
    let request = generate_request_from_json(&request_json).map_err(|err| Error::from_reason(err.to_string()))?;
    verify_response(verify_artifact_set_for_request(&set, &request))
}

/// Generate an archive for a lowercase variant id using a bare default topology.
#[napi]
pub fn generate_variant(variant: String) -> Result<GenerateResponse> {
    let kind = VariantKind::parse(&variant).ok_or_else(|| Error::from_reason(format!("unknown variant `{variant}`")))?;
    generate_response(GenerateRequest::bare(kind))
}

/// Generate from a full `zb.request/v0` JSON wire document.
#[napi]
pub fn generate_request_json(request_json: String) -> Result<GenerateResponse> {
    let request = generate_request_from_json(&request_json).map_err(|err| Error::from_reason(err.to_string()))?;
    generate_response(request)
}

/// Cooperative cancel flag shared with a libuv-pool generate task.
#[napi]
pub struct GenerateCancel {
    token: CancelToken,
}

#[napi]
impl GenerateCancel {
    /// Fresh flag. Call [`Self::cancel`] from the JavaScript thread.
    #[napi(constructor)]
    pub fn new() -> Self {
        Self { token: CancelToken::new() }
    }

    /// Request cancellation. Idempotent.
    #[napi]
    pub fn cancel(&self) {
        self.token.cancel();
    }
}

/// One JSON generate scheduled on the libuv thread pool.
pub struct GenerateJsonTask {
    /// `zb.request/v0` JSON document.
    request_json: String,
    /// Cooperative cancel flag cloned from [`GenerateCancel`].
    cancel: CancelToken,
}

#[napi]
impl Task for GenerateJsonTask {
    type Output = GenerateResponse;
    type JsValue = GenerateResponse;

    fn compute(&mut self) -> Result<Self::Output> {
        let request = generate_request_from_json(&self.request_json).map_err(|err| Error::from_reason(err.to_string()))?;
        generate_response_with_cancel(request, &self.cancel)
    }

    fn resolve(&mut self, _env: Env, output: Self::Output) -> Result<Self::JsValue> {
        Ok(output)
    }
}

/// Generate on the libuv thread pool so `AbortSignal` can flip [`GenerateCancel`].
#[napi]
pub fn generate_request_json_cancellable(request_json: String, cancel: &GenerateCancel) -> AsyncTask<GenerateJsonTask> {
    AsyncTask::new(GenerateJsonTask { request_json, cancel: cancel.token.clone() })
}
