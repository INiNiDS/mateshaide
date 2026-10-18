pub mod camera;
pub mod canvas;
pub mod color;
pub mod engine;
pub mod input;
pub mod palette;
pub mod render_pipeline;
pub mod renderer;
pub mod shape;
pub mod test_spawn;
pub mod utils;

use crate::color::Color;
use crate::engine::Engine;
use crate::input::{InputManager, InputUpdate, Shortcut};
use crate::shape::ShapeInstance;
use anyhow::Result;
use glam::Vec2;
use std::sync::Arc;
use tracing::info;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::KeyCode;
use winit::window::{Window, WindowId};

/// Main application struct.
struct App {
    window: Option<Arc<Window>>,
    engine: Option<Engine>,
    input: InputManager,
}

impl App {
    fn new() -> Self {
        let mut input = InputManager::new();

        initialize_input(&mut input);

        Self {
            window: None,
            engine: None,
            input,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let window = event_loop
            .create_window(Window::default_attributes().with_title("MateshaIDE"))
            .unwrap();
        let window = Arc::new(window);

        let mut engine = match pollster::block_on(Engine::new(window.clone())) {
            Ok(engine) => engine,
            Err(err) => {
                tracing::error!("failed to init engine: {err}");
                event_loop.exit();
                return;
            }
        };

        // Test
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

        self.window = Some(window);
        self.engine = Some(engine);

        info!("wgpu context and window successfully initialized");
    }

    // High Complexity
    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let (Some(engine), Some(window)) = (&mut self.engine, &self.window) else {
            return;
        };

        if window.id() != window_id {
            return;
        }

        match event {
            WindowEvent::CloseRequested => {
                info!("Window {:?} close requested", window_id);
                event_loop.exit();
                return;
            }
            WindowEvent::RedrawRequested => {
                let is_gliding = engine.update();
                engine.redraw();
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
                    engine.canvas.zoom(self.input.cursor_position(), delta.y)
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

fn initialize_input(input: &mut InputManager) {
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
    input.bind(Shortcut::key(KeyCode::Digit7).ctrl(), "test_next_picked_color");
}

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = App::new();
    event_loop.run_app(&mut app)?;

    Ok(())
}
