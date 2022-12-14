//! WebAssembly export surface for zb.

use wasm_bindgen::prelude::*;
use zb_generate::list_variants;

/// Comma-separated variant ids for capability probes.
#[wasm_bindgen(js_name = listVariants)]
pub fn list_variants_js() -> String {
    list_variants()
        .iter()
        .map(|v| v.as_str())
        .collect::<Vec<_>>()
        .join(",")
}
