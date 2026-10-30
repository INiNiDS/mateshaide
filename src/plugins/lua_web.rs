use super::{
    helpers::Helpers,
    runtime::{BOOTSTRAP_SOURCE, JSON_SOURCE},
};
use anyhow::{Context, Result};
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::{JsCast, prelude::*};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(catch,js_namespace=MateshaLua,js_name=create)]
    fn create_vm(
        host: &js_sys::Function,
        json: &str,
        bootstrap: &str,
    ) -> std::result::Result<u32, JsValue>;
    #[wasm_bindgen(catch,js_namespace=MateshaLua,js_name=invoke)]
    fn invoke_vm(
        id: u32,
        files: &str,
        entry: &str,
        state: &str,
        name: &str,
        args: &str,
    ) -> std::result::Result<String, JsValue>;
    #[wasm_bindgen(js_namespace=MateshaLua,js_name=destroy)]
    fn destroy_vm(id: u32);
}
pub struct Backend {
    id: u32,
    _host: Closure<dyn FnMut(String, String) -> std::result::Result<String, JsValue>>,
}
impl Backend {
    pub fn new() -> Result<Self> {
        let helpers = Rc::new(RefCell::new(Helpers::new()));
        let host = Closure::wrap(Box::new(
            move |operation: String, input: String| -> std::result::Result<String, JsValue> {
                Ok(helpers.borrow_mut().bridge(&operation, &input))
            },
        )
            as Box<dyn FnMut(String, String) -> std::result::Result<String, JsValue>>);
        let id = create_vm(host.as_ref().unchecked_ref(), JSON_SOURCE, BOOTSTRAP_SOURCE)
            .map_err(|e| anyhow::anyhow!("Browser Lua init: {e:?}"))?;
        Ok(Self { id, _host: host })
    }
    pub fn invoke(
        &mut self,
        files: &str,
        entry: &str,
        state: &str,
        name: &str,
        args: &str,
    ) -> Result<String> {
        invoke_vm(self.id, files, entry, state, name, args)
            .map_err(|e| anyhow::anyhow!("Browser Lua: {e:?}"))
    }
}
impl Drop for Backend {
    fn drop(&mut self) {
        destroy_vm(self.id);
    }
}

pub async fn prepare_browser() -> Result<()> {
    let promise = js_sys::Reflect::get(&js_sys::global(), &JsValue::from_str("MateshaLuaReady"))
        .map_err(|_| anyhow::anyhow!("Could not read MateshaLuaReady"))?
        .dyn_into::<js_sys::Promise>()
        .ok()
        .context("Missing web/lua-runtime.js. Load it before the Rust/Wasm application.")?;
    wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .map_err(|e| anyhow::anyhow!("Lua browser runtime failed to load: {e:?}"))?;
    Ok(())
}
