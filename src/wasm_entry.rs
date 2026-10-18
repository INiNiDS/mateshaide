use wasm_bindgen::prelude::*;
use winit::event_loop::{ControlFlow, EventLoop};

use crate::App;

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    tracing_wasm::set_as_global_default();

    use winit::platform::web::EventLoopExtWebSys;

    let event_loop = EventLoop::new().map_err(|e| JsValue::from_str(&e.to_string()))?;
    event_loop.set_control_flow(ControlFlow::Wait);

    event_loop
        .spawn_app(App::new())
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    Ok(())
}