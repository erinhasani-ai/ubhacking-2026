use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn version() -> String {
    aether_core::version().to_string()
}
