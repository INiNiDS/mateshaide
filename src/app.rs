use crate::color::Color;
use crate::engine::Engine;
use crate::input::{InputManager, InputUpdate, Shortcut};
use crate::palette;
use crate::shape::ShapeInstance;
use glam::Vec2;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use tracing::info;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::KeyCode;
use winit::window::{Window, WindowId};

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

        match event {
            WindowEvent::RedrawRequested => {
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
                return;
            }
            WindowEvent::Resized(new_size) => {
                if new_size.width > 0 && new_size.height > 0 {
                    engine.resize(new_size.width, new_size.height);
                    window.request_redraw();
                }
                return;
            }
            _ => {}
        }

        let update = self.input.process_event(&event, engine);

        match update {
            InputUpdate::ActionExecuted => {
                window.request_redraw();
            }
            InputUpdate::MouseButton { button, state } => {
                if button == MouseButton::Right || button == MouseButton::Middle {
                    match state {
                        ElementState::Pressed => {
                            if !engine.canvas.is_panning() {
                                engine.canvas.start_pan();
                            }
                        }
                        ElementState::Released => {
                            let still_holding = self.input.is_mouse_down(MouseButton::Middle)
                                || self.input.is_mouse_down(MouseButton::Right);
                            if !still_holding {
                                engine.canvas.end_pan();
                                engine.reset_frame_time();
                                window.request_redraw();
                            }
                        }
                    }
                }
            }
            InputUpdate::MouseWheel { delta } => {
                if self.input.is_ctrl() {
                    engine.canvas.zoom(self.input.cursor_position(), delta.y);
                } else {
                    let pan_delta = if self.input.is_shift() {
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
            InputUpdate::CursorMoved { delta, position } => {
                engine.test_spawner.cursor_screen = position;
                let is_panning = self.input.is_mouse_down(MouseButton::Right)
                    || self.input.is_mouse_down(MouseButton::Middle);
                if is_panning {
                    engine.canvas.track_pan(delta);
                    window.request_redraw();
                }
            }
            InputUpdate::FocusLost => {
                if engine.canvas.is_panning() {
                    engine.canvas.cancel_pan();
                }
                engine.reset_frame_time();
                window.request_redraw();
            }
            InputUpdate::None => {}
        }
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
}

/// Перенос старого `initialize_input` из main.rs БЕЗ изменения содержимого.
pub fn initialize_input(input: &mut InputManager) {
    input.register_action("test_toggle_spawn", |engine: &mut Engine| {
        engine.toggle_test_spawn();
    });
    input.register_action("test_select_rect", |engine: &mut Engine| {
        engine.select_test_shape("rect");
    });
    input.register_action("test_select_circle", |engine: &mut Engine| {
        engine.select_test_shape("circle");
    });
    input.register_action("test_select_hexagon", |engine: &mut Engine| {
        engine.select_test_shape("hexagon");
    });
    input.register_action("test_select_rounded_rect", |engine: &mut Engine| {
        engine.select_test_shape("rounded_rect");
    });
    input.register_action("test_spawn", |engine: &mut Engine| {
        engine.spawn_test_shape();
    });

    input.bind(Shortcut::key(KeyCode::Digit1).ctrl(), "test_toggle_spawn");
    input.bind(Shortcut::key(KeyCode::Digit2).ctrl(), "test_select_rect");
    input.bind(Shortcut::key(KeyCode::Digit3).ctrl(), "test_select_circle");
    input.bind(Shortcut::key(KeyCode::Digit4).ctrl(), "test_select_hexagon");
    input.bind(
        Shortcut::key(KeyCode::Digit5).ctrl(),
        "test_select_rounded_rect",
    );
    input.bind(Shortcut::key(KeyCode::Digit1), "test_spawn");

    input.register_action("zoom_in", |engine: &mut Engine| {
        engine.canvas.zoom_at_center(1.0)
    });
    input.register_action("zoom_out", |engine: &mut Engine| {
        engine.canvas.zoom_at_center(-1.0)
    });
    input.register_action("reset_view", |engine: &mut Engine| {
        engine.canvas.reset_view()
    });

    input.bind(Shortcut::key(KeyCode::Equal).ctrl(), "zoom_in");
    input.bind(Shortcut::key(KeyCode::NumpadAdd).ctrl(), "zoom_in");
    input.bind(Shortcut::key(KeyCode::Equal).ctrl().shift(), "zoom_in");

    input.bind(Shortcut::key(KeyCode::Minus).ctrl(), "zoom_out");
    input.bind(Shortcut::key(KeyCode::NumpadSubtract).ctrl(), "zoom_out");

    input.bind(Shortcut::key(KeyCode::Digit0).ctrl(), "reset_view");
    input.bind(Shortcut::key(KeyCode::Numpad0).ctrl(), "reset_view");

    input.register_action("test_toggle_color_mode", |engine: &mut Engine| {
        engine.toggle_test_color_mode();
    });
    input.bind(
        Shortcut::key(KeyCode::Digit6).ctrl(),
        "test_toggle_color_mode",
    );

    input.register_action("test_next_picked_color", |engine: &mut Engine| {
        engine.next_test_picked_color();
    });
    input.bind(
        Shortcut::key(KeyCode::Digit7).ctrl(),
        "test_next_picked_color",
    );

    input.register_action("undo", |engine: &mut Engine| {
        engine.undo();
    });
    input.register_action("redo", |engine: &mut Engine| {
        engine.redo();
    });

    input.bind(Shortcut::key(KeyCode::KeyZ).ctrl(), "undo");
    input.bind(Shortcut::key(KeyCode::KeyX).ctrl(), "redo");
    input.bind(Shortcut::key(KeyCode::KeyY).ctrl(), "redo");
    input.bind(Shortcut::key(KeyCode::KeyZ).ctrl().shift(), "redo");

    input.register_action("test_toggle_eraser", |engine: &mut Engine| {
        engine.toggle_test_eraser();
    });
    input.bind(Shortcut::key(KeyCode::Digit8).ctrl(), "test_toggle_eraser");
}
