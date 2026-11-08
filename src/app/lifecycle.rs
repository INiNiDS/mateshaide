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
        let Some(window) = self.window_for(window_id) else {
            return;
        };

        if matches!(event, WindowEvent::CloseRequested) {
            self.handle_close(event_loop, window_id);
            return;
        }

        let engine_cell = Rc::clone(&self.engine);
        let mut engine_guard = engine_cell.borrow_mut();
        let Some(engine) = engine_guard.as_mut() else {
            return;
        };

        if Self::handle_system_event(engine, &window, &event) {
            return;
        }

        if engine.is_capturing() {
            self.handle_captured_event(engine, &window, &event);
        } else {
            self.handle_regular_event(engine, &window, &event);
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let Ok(mut slot) = self.engine.try_borrow_mut() else {
            return;
        };
        let Some(engine) = slot.as_mut() else {
            return;
        };
        let (next, redraw) = engine.plugin_tick();
        event_loop.set_control_flow(next.map_or(
            winit::event_loop::ControlFlow::Wait,
            winit::event_loop::ControlFlow::WaitUntil,
        ));
        if redraw && let Some(window) = self.window.borrow().as_ref() {
            window.request_redraw();
        }
    }
}

impl App {
    fn window_for(&self, window_id: WindowId) -> Option<Arc<Window>> {
        self.window
            .borrow()
            .as_ref()
            .filter(|window| window.id() == window_id)
            .cloned()
    }

    fn handle_close(&self, event_loop: &ActiveEventLoop, window_id: WindowId) {
        info!("Window {window_id:?} close requested");
        if let Ok(mut slot) = self.engine.try_borrow_mut()
            && let Some(engine) = slot.as_mut()
        {
            engine.plugins_before_exit();
        }
        event_loop.exit();
    }

    fn handle_system_event(engine: &mut Engine, window: &Arc<Window>, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::RedrawRequested => {
                Self::handle_redraw_requested(window, engine);
                true
            }
            WindowEvent::Resized(size) => {
                Self::handle_window_resized(window, engine, size.width, size.height);
                true
            }
            _ => false,
        }
    }

    fn process_input_without_actions(
        &mut self,
        engine: &mut Engine,
        window: &Arc<Window>,
        event: &WindowEvent,
    ) {
        let update = self.input.process_event_without_actions(event);
        if matches!(update, InputUpdate::FocusLost) {
            Self::handle_input_update(&self.input, engine, update, window);
        }
    }

    fn handle_captured_event(
        &mut self,
        engine: &mut Engine,
        window: &Arc<Window>,
        event: &WindowEvent,
    ) {
        self.process_input_without_actions(engine, window, event);

        if let WindowEvent::CursorMoved { position, .. } = event {
            engine.capture_cursor = Vec2::new(position.x as f32, position.y as f32);
        }

        Self::handle_capture_event(engine, event, window);
    }

    fn handle_regular_event(
        &mut self,
        engine: &mut Engine,
        window: &Arc<Window>,
        event: &WindowEvent,
    ) {
        if engine.handle_plugin_event(event, self.input.cursor_position(), self.input.modifiers()) {
            self.process_input_without_actions(engine, window, event);
            window.request_redraw();
            return;
        }

        let update = self.input.process_event(event, engine);
        Self::handle_input_update(&self.input, engine, update, window);
        self.sync_cursor(engine, window);
    }

    fn sync_cursor(&mut self, engine: &Engine, window: &Arc<Window>) {
        let cursor = engine.cursor_icon();
        if cursor != self.cursor {
            self.cursor = cursor;
            window.set_cursor(cursor);
        }
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
    let attributes = Window::default_attributes()
        .with_title("MateshaIDE")
        .with_window_icon(app_icon());

    Arc::new(event_loop.create_window(attributes).unwrap())
}

#[cfg(not(target_arch = "wasm32"))]
fn app_icon() -> Option<winit::window::Icon> {
    let image = image::load_from_memory(include_bytes!("../../assets/logo.png"))
        .ok()?
        .into_rgba8();
    let (width, height) = image.dimensions();
    winit::window::Icon::from_rgba(image.into_raw(), width, height).ok()
}
