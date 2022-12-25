//! WebAssembly export surface for zb (isomorphic with Node public surface).

#![warn(missing_docs)]

use wasm_bindgen::prelude::*;
use zb_generate::{capabilities, generate, list_variants, report_json};
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
