
#[cfg(target_arch = "wasm32")]
pub mod js_bridge {
    #[wasm_bindgen::prelude::wasm_bindgen]
    extern "C" {
        #[wasm_bindgen::prelude::wasm_bindgen(js_name = showFallback)]
        pub fn show_fallback(reason: &str);
    }
}
