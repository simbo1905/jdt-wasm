use serde_json::Value;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn transform(source_json: &str, transform_json: &str) -> Result<String, JsValue> {
    let source: Value =
        serde_json::from_str(source_json).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let transform: Value =
        serde_json::from_str(transform_json).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let result =
        jdt_codegen::apply(&source, &transform).map_err(|e| JsValue::from_str(&e.to_string()))?;
    serde_json::to_string(&result).map_err(|e| JsValue::from_str(&e.to_string()))
}
