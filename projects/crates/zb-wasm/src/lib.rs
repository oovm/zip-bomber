//! WebAssembly export surface for zb (isomorphic with Node public surface).

#![warn(missing_docs)]

use wasm_bindgen::prelude::*;
use zb_generate::{
    capabilities, generate, list_variants, plan_generate, report_json, verify_zip_bytes,
};
use zb_types::{GenerateRequest, Outcome, VariantKind};

/// Comma-separated variant ids for capability probes.
#[wasm_bindgen(js_name = listVariants)]
pub fn list_variants_js() -> String {
    list_variants()
        .iter()
        .map(|v| v.as_str())
        .collect::<Vec<_>>()
        .join(",")
}

/// JSON capability matrix (`CapabilitySet`).
#[wasm_bindgen(js_name = capabilitiesJson)]
pub fn capabilities_json() -> Result<String, JsValue> {
    serde_json::to_string(&capabilities()).map_err(|err| JsValue::from_str(&err.to_string()))
}

fn root_bytes(set: &zb_types::ArtifactSet) -> Vec<u8> {
    set.artifacts
        .iter()
        .find(|a| a.id == set.root)
        .or_else(|| set.artifacts.last())
        .map(|a| a.bytes.clone())
        .unwrap_or_default()
}

fn bytes_to_b64(bytes: &[u8]) -> String {
    // Minimal base64 (no external crate) for isomorphic wire bytes.
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[((n >> 18) & 63) as usize] as char);
        out.push(TABLE[((n >> 12) & 63) as usize] as char);
        out.push(if chunk.len() > 1 {
            TABLE[((n >> 6) & 63) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            TABLE[(n & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}

/// Plan a bare variant and return JSON `{ ok, feasible?, summary?, profile?, report, error? }`.
#[wasm_bindgen(js_name = planVariantJson)]
pub fn plan_variant_json(variant: String) -> Result<String, JsValue> {
    let kind = VariantKind::parse(&variant)
        .ok_or_else(|| JsValue::from_str(&format!("unknown variant `{variant}`")))?;
    let request = GenerateRequest::bare(kind);
    let payload = match plan_generate(request) {
        Outcome::Ok { value, report, .. } => {
            let report = report_json(&report).map_err(|err| JsValue::from_str(&err.to_string()))?;
            serde_json::json!({
                "ok": true,
                "feasible": value.feasible,
                "summary": value.summary,
                "profile": value.profile,
                "variant": value.variant,
                "report": serde_json::from_str::<serde_json::Value>(&report).unwrap_or(serde_json::Value::Null)
            })
        }
        Outcome::Err { error, report, .. } => {
            let report = report_json(&report).map_err(|err| JsValue::from_str(&err.to_string()))?;
            serde_json::json!({
                "ok": false,
                "feasible": false,
                "error": {
                    "category": error.category.as_str(),
                    "code": error.code.as_str(),
                    "message": error.message,
                },
                "report": serde_json::from_str::<serde_json::Value>(&report).unwrap_or(serde_json::Value::Null)
            })
        }
    };
    serde_json::to_string(&payload).map_err(|err| JsValue::from_str(&err.to_string()))
}

fn b64_to_bytes(b64: &str) -> Result<Vec<u8>, JsValue> {
    const TABLE: &[u8] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut values = Vec::with_capacity(b64.len());
    for ch in b64.bytes() {
        if ch == b'=' {
            break;
        }
        let idx = TABLE
            .iter()
            .position(|b| *b == ch)
            .ok_or_else(|| JsValue::from_str("invalid base64"))?;
        values.push(idx as u8);
    }
    let mut out = Vec::new();
    for chunk in values.chunks(4) {
        let v0 = u32::from(*chunk.first().unwrap_or(&0));
        let v1 = u32::from(*chunk.get(1).unwrap_or(&0));
        let v2 = u32::from(*chunk.get(2).unwrap_or(&0));
        let v3 = u32::from(*chunk.get(3).unwrap_or(&0));
        let n = (v0 << 18) | (v1 << 12) | (v2 << 6) | v3;
        out.push(((n >> 16) & 0xff) as u8);
        if chunk.len() > 2 {
            out.push(((n >> 8) & 0xff) as u8);
        }
        if chunk.len() > 3 {
            out.push((n & 0xff) as u8);
        }
    }
    Ok(out)
}

/// Verify ZIP bytes (base64) and return JSON verification report.
#[wasm_bindgen(js_name = verifyZipBytesJson)]
pub fn verify_zip_bytes_json(bytes_base64: String) -> Result<String, JsValue> {
    let bytes = b64_to_bytes(&bytes_base64)?;
    let payload = match verify_zip_bytes(&bytes) {
        Ok(report) => serde_json::json!({
            "ok": report.ok,
            "report": report,
        }),
        Err(error) => serde_json::json!({
            "ok": false,
            "error": {
                "category": error.category.as_str(),
                "code": error.code.as_str(),
                "message": error.message,
            }
        }),
    };
    serde_json::to_string(&payload).map_err(|err| JsValue::from_str(&err.to_string()))
}

/// Generate a bare variant and return JSON `{ ok, report, bytesBase64?, error? }`.
#[wasm_bindgen(js_name = generateVariantJson)]
pub fn generate_variant_json(variant: String) -> Result<String, JsValue> {
    let kind = VariantKind::parse(&variant)
        .ok_or_else(|| JsValue::from_str(&format!("unknown variant `{variant}`")))?;
    let request = GenerateRequest::bare(kind);
    let payload = match generate(request) {
        Outcome::Ok { value, report, .. } => {
            let report = report_json(&report).map_err(|err| JsValue::from_str(&err.to_string()))?;
            let bytes = root_bytes(&value);
            serde_json::json!({
                "ok": true,
                "bytesBase64": bytes_to_b64(&bytes),
                "byteLength": bytes.len(),
                "root": value.root.0,
                "report": serde_json::from_str::<serde_json::Value>(&report).unwrap_or(serde_json::Value::Null)
            })
        }
        Outcome::Err { error, report, .. } => {
            let report = report_json(&report).map_err(|err| JsValue::from_str(&err.to_string()))?;
            serde_json::json!({
                "ok": false,
                "error": {
                    "category": error.category.as_str(),
                    "code": error.code.as_str(),
                    "message": error.message,
                },
                "report": serde_json::from_str::<serde_json::Value>(&report).unwrap_or(serde_json::Value::Null)
            })
        }
    };
    serde_json::to_string(&payload).map_err(|err| JsValue::from_str(&err.to_string()))
}
