//! WebAssembly export surface for zb (isomorphic with Node public surface).

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

/// Generate a bare variant and return JSON `{ ok, report, error? }`.
#[wasm_bindgen(js_name = generateVariantJson)]
pub fn generate_variant_json(variant: String) -> Result<String, JsValue> {
    let kind = VariantKind::parse(&variant)
        .ok_or_else(|| JsValue::from_str(&format!("unknown variant `{variant}`")))?;
    let request = GenerateRequest::bare(kind);
    let payload = match generate(request) {
        Outcome::Ok { report, .. } => {
            let report = report_json(&report).map_err(|err| JsValue::from_str(&err.to_string()))?;
            serde_json::json!({ "ok": true, "report": serde_json::from_str::<serde_json::Value>(&report).unwrap_or(serde_json::Value::Null) })
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
