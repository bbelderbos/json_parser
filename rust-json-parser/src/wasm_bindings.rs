use crate::parse;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn parse_json_wasm(input: &str) -> Result<String, JsValue> {
    parse(input)
        .map(|value| value.pretty_print(2))
        .map_err(|e| JsValue::from_str(&e.to_string()))
}
