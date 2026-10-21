use crate::{App, Engine, InputManager, InputUpdate};
use glam::Vec2;
use winit::event::{ElementState, MouseButton};
use winit::window::Window;

impl App {
    pub(super) fn handle_redraw_requested(window: &Window, engine: &mut Engine) {
        #[allow(unused_mut)]
        // mut needs to be mutable in wasm32, but when you're running on desktop it gives a warning
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

    pub(super) fn handle_window_resized(
        window: &Window,
        engine: &mut Engine,
        width: u32,
        height: u32,
    ) {
        if width > 0 && height > 0 {
            engine.resize(width, height);
            window.request_redraw();
        }
    }

    pub(super) fn handle_input_update(
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
        delta: Vec2,
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
