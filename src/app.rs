use crate::color::Color;
use crate::engine::Engine;
use crate::input::{InputManager, InputUpdate};
use crate::palette;
use crate::shape::ShapeInstance;
use crate::text::CanvasText;
use glam::Vec2;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use tracing::info;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};
use crate::builtin_input::initialize_input;

type Slot<T> = Rc<RefCell<Option<T>>>;

#[cfg(target_arch = "wasm32")]
mod js_bridge {
    #[wasm_bindgen::prelude::wasm_bindgen]
    extern "C" {
        #[wasm_bindgen::prelude::wasm_bindgen(js_name = showFallback)]
        pub fn show_fallback(reason: &str);
    }
}

pub struct App {
    window: Slot<Arc<Window>>,
    engine: Slot<Engine>,
    input: InputManager,
}

impl App {
    pub fn new() -> Self {
        let mut input = InputManager::new();
        initialize_input(&mut input);
        Self {
            window: Rc::new(RefCell::new(None)),
            engine: Rc::new(RefCell::new(None)),
            input,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.borrow().is_some() {
            return;
        }

        let window = {
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
                    .and_then(|v| v.as_f64())
                    .unwrap_or(800.0);
                let inner_height = window
                    .inner_height()
                    .ok()
                    .and_then(|v| v.as_f64())
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
        };
        *self.window.borrow_mut() = Some(window.clone());

        let engine_slot = self.engine.clone();
        let window_slot = self.window.clone();
        let window_for_init = window.clone();

        let init = async move {
            match Engine::new(window_for_init).await {
                Ok(mut engine) => {
                    seed_demo_shapes(&mut engine);
                    *engine_slot.borrow_mut() = Some(engine);
                    if let Some(w) = window_slot.borrow().as_ref() {
                        w.request_redraw();
                    }
                    info!("wgpu context and window successfully initialized");
                }
                Err(err) => {
                    tracing::error!("failed to init engine: {err}");
                    #[cfg(target_arch = "wasm32")]
                    js_bridge::show_fallback(&format!("engine init failed: {err}"));
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
            Some(w) if w.id() == window_id => w.clone(),
            _ => return,
        };

        if matches!(event, WindowEvent::CloseRequested) {
            info!("Window {:?} close requested", window_id);
            event_loop.exit();
            return;
        }

        let mut engine_guard = self.engine.borrow_mut();
        let engine = match engine_guard.as_mut() {
            Some(e) => e,
            None => return,
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

        let update = self.input.process_event(&event, engine);

        Self::handle_input_update(&self.input, engine, update, &window)
    }
}

impl App {
    fn handle_redraw_requested(window: &Window, engine: &mut Engine) {
        #[allow(unused_mut)] // mut needs to be mutable in wasm32, but when you're running on desktop it gives a warning
        let mut current = window.inner_size();

        #[cfg(target_arch = "wasm32")]
        {
            if let Some(win) = web_sys::window() {
                let dpr = win.device_pixel_ratio();

                if let (Ok(w), Ok(h)) = (win.inner_width(), win.inner_height()) {
                    if let (Some(w_val), Some(h_val)) = (w.as_f64(), h.as_f64()) {
                        let real_w = (w_val * dpr).round() as u32;
                        let real_h = (h_val * dpr).round() as u32;

                        if real_w > 0 && real_h > 0 {
                            current.width = real_w;
                            current.height = real_h;
                        }
                    }
                }
            }
        }

        let engine_size = engine.size();

        if current.width > 0
            && current.height > 0
            && (current.width != engine_size.width || current.height != engine_size.height)
        {
            engine.resize(current.width, current.height);
        }

        let is_gliding = engine.update();

        if current.width > 0 && current.height > 0 {
            engine.redraw();
        }

        if is_gliding {
            window.request_redraw();
        }
    }

    fn handle_window_resized(window: &Window, engine: &mut Engine, width: u32, height: u32) {
        if width > 0 && height > 0 {
            engine.resize(width, height);
            window.request_redraw();
        }
    }

    fn handle_input_update(
        input: &InputManager,
        engine: &mut Engine,
        update: InputUpdate,
        window: &Window,
    ) {
        match update {
            InputUpdate::ActionExecuted => {
                window.request_redraw();
            }
            InputUpdate::None => {}
            InputUpdate::FocusLost => {
                Self::handle_focus_lost(engine, window);
            }
            InputUpdate::CursorMoved { position, delta } => {
                Self::handle_cursor_moved(input, engine, position, window, delta);
            }
            InputUpdate::MouseWheel { delta } => {
                Self::handle_mouse_wheel(input, engine, delta, window);
            }
            InputUpdate::MouseButton { button, state } => {
                Self::handle_mouse_button(input, engine, button, state, window);
            }
        }
    }

    fn handle_mouse_button(
        input: &InputManager,
        engine: &mut Engine,
        button: MouseButton,
        state: ElementState,
        window: &Window,
    ) {
        if button == MouseButton::Left {
            match state {
                ElementState::Pressed => {
                    engine
                        .draw
                        .begin(&mut engine.canvas, input.cursor_position());
                    window.request_redraw();
                }
                ElementState::Released => {
                    engine.draw.end(&mut engine.canvas);
                    window.request_redraw();
                }
            }
        }
        if button == MouseButton::Right || button == MouseButton::Middle {
            match state {
                ElementState::Pressed => {
                    if !engine.canvas.is_panning() {
                        engine.canvas.start_pan();
                    }
                }
                ElementState::Released => {
                    let still_holding = input.is_mouse_down(MouseButton::Middle)
                        || input.is_mouse_down(MouseButton::Right);
                    if !still_holding {
                        engine.canvas.end_pan();
                        engine.reset_frame_time();
                        window.request_redraw();
                    }
                }
            }
        }
    }

    fn handle_mouse_wheel(input: &InputManager, engine: &mut Engine, delta: Vec2, window: &Window) {
        if input.is_ctrl() {
            engine.canvas.zoom(input.cursor_position(), delta.y);
        } else {
            let pan_delta = if input.is_shift() {
                let x = if delta.x.abs() > 0.001 {
                    delta.x
                } else {
                    delta.y
                };
                Vec2::new(x * 32.0, 0.0)
            } else {
                Vec2::new(delta.x * 32.0, delta.y * 32.0)
            };
            engine.canvas.pan(pan_delta);
        }
        window.request_redraw();
    }

    fn handle_cursor_moved(
        input: &InputManager,
        engine: &mut Engine,
        position: Vec2,
        window: &Window,
        delta: Vec2
    ) {
        engine.test_spawner.cursor_screen = position;

        if engine.canvas.is_stroking() {
            engine.draw.point(&mut engine.canvas, position);
            window.request_redraw();
        }
        let is_panning =
            input.is_mouse_down(MouseButton::Right) || input.is_mouse_down(MouseButton::Middle);
        if is_panning {
            engine.canvas.track_pan(delta);
            window.request_redraw();
        }
    }

    fn handle_focus_lost(engine: &mut Engine, window: &Window) {
        engine.draw.end(&mut engine.canvas);
        if engine.canvas.is_panning() {
            engine.canvas.cancel_pan();
        }
        engine.reset_frame_time();
        window.request_redraw();
    }
}

fn seed_demo_shapes(engine: &mut Engine) {
    engine.canvas.add_shape(ShapeInstance::rect(
        -50.0,
        -50.0,
        100.0,
        100.0,
        palette::RED_500,
    ));
    engine.canvas.add_shape(ShapeInstance::rect(
        120.0,
        60.0,
        180.0,
        90.0,
        Color::hex(0x22c55e),
    ));
    engine.canvas.add_shape(ShapeInstance::rect(
        -300.0,
        -150.0,
        80.0,
        300.0,
        palette::BLUE_500,
    ));
    engine.canvas.add_shape(ShapeInstance::rect(
        200.0,
        -200.0,
        60.0,
        60.0,
        palette::VIOLET_500,
    ));
    engine
        .canvas
        .add_shape(ShapeInstance::circle(0.0, 200.0, 40.0, palette::INFO));

    engine.canvas.add_text(CanvasText {
        position: Vec2::new(-50.0, -80.0),
        text: "Hello, MateshaIDE!".to_owned(),
        size: 24.0,
        color: palette::GRAY_900,
    });
}


