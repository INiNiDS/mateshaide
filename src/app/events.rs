use crate::graphics::capture_overlay::ButtonAction;
use crate::{App, Engine, InputManager, InputUpdate};
use glam::Vec2;
use winit::event::{ElementState, MouseButton};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::Window;

impl App {
    pub(super) fn handle_redraw_requested(window: &Window, engine: &mut Engine) {
        #[allow(unused_mut)]
        let mut current = window.inner_size();

        #[cfg(target_arch = "wasm32")]
        {
            if let Some(win) = web_sys::window() {
                let dpr = win.device_pixel_ratio();
                if let (Ok(w), Ok(h)) = (win.inner_width(), win.inner_height()) {
                    if let (Some(wv), Some(hv)) = (w.as_f64(), h.as_f64()) {
                        let rw = (wv * dpr).round() as u32;
                        let rh = (hv * dpr).round() as u32;
                        if rw > 0 && rh > 0 {
                            current.width = rw;
                            current.height = rh;
                        }
                    }
                }
            }
        }

        let es = engine.size();
        if current.width > 0
            && current.height > 0
            && (current.width != es.width || current.height != es.height)
        {
            engine.resize(current.width, current.height);
        }

        let gliding = engine.update();
        if current.width > 0 && current.height > 0 {
            engine.redraw();
        }
        if gliding {
            window.request_redraw();
        }
    }

    pub(super) fn handle_window_resized(window: &Window, engine: &mut Engine, w: u32, h: u32) {
        if w > 0 && h > 0 {
            engine.resize(w, h);
            window.request_redraw();
        }
    }

    // HIGH COMPLEXITY TODO
    pub(super) fn handle_capture_event(
        engine: &mut Engine,
        event: &winit::event::WindowEvent,
        window: &Window,
    ) {
        match event {
            winit::event::WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed {
                    if let PhysicalKey::Code(key) = event.physical_key {
                        match key {
                            KeyCode::Escape => {
                                engine.dismiss_capture();
                                window.request_redraw();
                            }
                            KeyCode::KeyS if engine.is_in_capture_preview() => {
                                engine.save_capture();
                                window.request_redraw();
                            }
                            KeyCode::KeyC if engine.is_in_capture_preview() => {
                                engine.copy_capture();
                                window.request_redraw();
                            }
                            _ => {}
                        }
                    }
                }
            }

            winit::event::WindowEvent::MouseInput { state, button, .. } => {
                if *state == ElementState::Pressed && *button == MouseButton::Left {
                    let cursor = engine.capture_cursor;
                    match engine.check_button_click(cursor) {
                        Some(ButtonAction::Save) => {
                            engine.save_capture();
                            window.request_redraw();
                        }
                        Some(ButtonAction::Copy) => {
                            engine.copy_capture();
                            window.request_redraw();
                        }
                        None => {}
                    }
                }
            }

            _ => {}
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
            InputUpdate::FocusLost => Self::handle_focus_lost(engine, window),
            InputUpdate::CursorMoved { position, delta } => {
                Self::handle_cursor_moved(input, engine, position, window, delta)
            }
            InputUpdate::MouseWheel { delta } => {
                Self::handle_mouse_wheel(input, engine, delta, window)
            }
            InputUpdate::MouseButton { button, state } => {
                Self::handle_mouse_button(input, engine, button, state, window)
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
                    let still = input.is_mouse_down(MouseButton::Middle)
                        || input.is_mouse_down(MouseButton::Right);
                    if !still {
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
            let pd = if input.is_shift() {
                let x = if delta.x.abs() > 0.001 {
                    delta.x
                } else {
                    delta.y
                };
                Vec2::new(x * 32.0, 0.0)
            } else {
                Vec2::new(delta.x * 32.0, delta.y * 32.0)
            };
            engine.canvas.pan(pd);
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
        let panning =
            input.is_mouse_down(MouseButton::Right) || input.is_mouse_down(MouseButton::Middle);
        if panning {
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
