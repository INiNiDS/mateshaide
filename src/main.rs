pub mod camera;
pub mod engine;
pub mod render_pipeline;
pub mod renderer;

use crate::engine::Engine;
use anyhow::Result;
use glam::Vec2;
use std::sync::Arc;
use tracing::info;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

/// Main application struct.
struct App {
    window: Option<Arc<Window>>,
    engine: Option<Engine>,
    cursor_pos: Vec2,
}

impl App {
    fn new() -> Self {
        Self {
            window: None,
            engine: None,
            cursor_pos: Vec2::ZERO,
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
        let engine = pollster::block_on(Engine::new(window.clone()));

        self.window = Some(window);

        self.engine = Some(engine);

        info!("wgpu context and window successfully initialized");
    }

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
            }
            WindowEvent::RedrawRequested => {
                let is_gliding = engine.update();
                engine.redraw();
                if is_gliding {
                    window.request_redraw();
                }
            }
            WindowEvent::Resized(new_size) => {
                if new_size.width > 0 && new_size.height > 0 {
                    engine.resize(new_size.width, new_size.height);
                    window.request_redraw();
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let new_pos = Vec2::new(position.x as f32, position.y as f32);
                let delta = new_pos - self.cursor_pos;
                self.cursor_pos = new_pos;
                if  engine.is_panning() {
                    engine.track_pan(delta);
                    window.request_redraw();
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if button == winit::event::MouseButton::Right {
                    match state {
                        ElementState::Pressed => {
                            engine.start_pan();
                        }
                        ElementState::Released => {
                            engine.end_pan();
                            window.request_redraw();
                        }
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                // TODO: Make zoom with Ctrl + MouseWheel instead of just MouseWheel (and Ctrl + +, Ctrl + -)
                let scroll_data = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => y,
                    winit::event::MouseScrollDelta::PixelDelta(pos) => (pos.y as f32) * 0.05,
                };
                engine.zoom(self.cursor_pos, scroll_data);
                window.request_redraw();
            }
            _ => {}
        }
    }
}

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = App::new();
    event_loop.run_app(&mut app)?;

    Ok(())
}
