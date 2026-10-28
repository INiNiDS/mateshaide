use super::App;
#[cfg(debug_assertions)]
use super::demo;
use crate::InputUpdate;
use crate::engine::Engine;
use glam::Vec2;
use std::rc::Rc;
use std::sync::Arc;
use tracing::info;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.borrow().is_some() {
            return;
        }

        let window = create_window(event_loop);
        *self.window.borrow_mut() = Some(Arc::clone(&window));

        let engine_slot = Rc::clone(&self.engine);
        let window_slot = Rc::clone(&self.window);
        let window_for_init = Arc::clone(&window);

        let init = async move {
            match Engine::new(window_for_init).await {
                Ok(engine) => {
                    #[cfg(debug_assertions)]
                    let engine = {
                        let mut engine = engine;
                        demo::seed_demo_shapes(&mut engine);
                        engine
                    };

                    *engine_slot.borrow_mut() = Some(engine);

                    if let Some(window) = window_slot.borrow().as_ref() {
                        window.request_redraw();
                    }

                    info!("wgpu context and window successfully initialized");
                }
                Err(err) => {
                    tracing::error!("failed to init engine: {err}");

                    #[cfg(target_arch = "wasm32")]
                    crate::wasm::bridge::js_bridge::show_fallback(&format!(
                        "engine init failed: {err}"
                    ));
                }
            }
        };

        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(init);

        #[cfg(not(target_arch = "wasm32"))]
        pollster::block_on(init);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let window = match self.window.borrow().as_ref() {
            Some(window) if window.id() == window_id => Arc::clone(window),
            _ => return,
        };

        if matches!(&event, WindowEvent::CloseRequested) {
            info!("Window {window_id:?} close requested");
            event_loop.exit();
            return;
        }

        let mut engine_guard = self.engine.borrow_mut();
        let Some(engine) = engine_guard.as_mut() else {
            return;
        };

        match &event {
            WindowEvent::RedrawRequested => {
                Self::handle_redraw_requested(&window, engine);
                return;
            }
            WindowEvent::Resized(size) => {
                Self::handle_window_resized(&window, engine, size.width, size.height);
                return;
            }
            _ => {}
        }

        if engine.is_capturing() {
            let update = self.input.process_event_without_actions(&event);
            if matches!(update, InputUpdate::FocusLost) {
                Self::handle_input_update(&self.input, engine, update, &window);
            }

            if let WindowEvent::CursorMoved { position, .. } = &event {
                engine.capture_cursor = Vec2::new(position.x as f32, position.y as f32);
            }

            Self::handle_capture_event(engine, &event, &window);
            return;
        }

        let update = self.input.process_event(&event, engine);

        Self::handle_input_update(&self.input, engine, update, &window);
    }
}

fn create_window(event_loop: &ActiveEventLoop) -> Arc<Window> {
    #[cfg(target_arch = "wasm32")]
    let attributes = {
        use wasm_bindgen::JsCast;
        use winit::dpi::LogicalSize;
        use winit::platform::web::WindowAttributesExtWebSys;

        let window = web_sys::window().expect("no global window");
        let document = window.document().expect("no document");

        let canvas = document
            .get_element_by_id("matesha-canvas")
            .expect("missing #matesha-canvas in DOM")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("#matesha-canvas is not a <canvas>");

        let dpr = window.device_pixel_ratio();

        let inner_width = window
            .inner_width()
            .ok()
            .and_then(|value| value.as_f64())
            .unwrap_or(800.0);

        let inner_height = window
            .inner_height()
            .ok()
            .and_then(|value| value.as_f64())
            .unwrap_or(600.0);

        canvas.set_width((inner_width * dpr).max(1.0) as u32);
        canvas.set_height((inner_height * dpr).max(1.0) as u32);

        Window::default_attributes()
            .with_title("MateshaIDE")
            .with_canvas(Some(canvas))
            .with_inner_size(LogicalSize::new(inner_width, inner_height))
    };

    #[cfg(not(target_arch = "wasm32"))]
    let attributes = Window::default_attributes().with_title("MateshaIDE");

    Arc::new(event_loop.create_window(attributes).unwrap())
}
