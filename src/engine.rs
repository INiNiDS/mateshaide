use crate::renderer::Renderer;
use glam::Vec2;
use std::sync::Arc;
use std::time::Instant;
use winit::dpi::PhysicalSize;
use winit::window::Window;

// Struct for all engine components.
pub struct Engine {
    renderer: Renderer,
    pan_velocity: Vec2,
    last_frame_time: Instant,
    last_mouse_move: Instant,
    is_panning: bool,
}

impl Engine {
    pub async fn new(window: Arc<Window>) -> Self {
        Self {
            renderer: Renderer::new(window).await,
            pan_velocity: Vec2::ZERO,
            last_frame_time: Instant::now(),
            last_mouse_move: Instant::now(),
            is_panning: false,
        }
    }

    pub fn start_pan(&mut self) {
        self.is_panning = true;
        self.pan_velocity = Vec2::ZERO;
        self.last_mouse_move = Instant::now();
    }

    pub fn is_panning(&self) -> bool {
        self.is_panning
    }

    pub fn track_pan(&mut self, delta: Vec2) {
        if !self.is_panning {
            return;
        }

        let now = Instant::now();
        let dt = (now - self.last_mouse_move).as_secs_f32();
        self.last_mouse_move = now;

        if dt > 0.0001 {
            let instant_velocity = delta / dt;
            self.pan_velocity = self.pan_velocity.lerp(instant_velocity, 0.4);
        }

        self.pan(delta)
    }

    pub fn end_pan(&mut self) {
        self.is_panning = false;
        if self.last_mouse_move.elapsed().as_millis() > 60 {
            self.pan_velocity = Vec2::ZERO;
        }
    }

    pub fn redraw(&mut self) {
        self.renderer.redraw();
    }

    pub fn update(&mut self) -> bool {
        let now = Instant::now();
        let dt = (now - self.last_frame_time).as_secs_f32().min(0.1);
        self.last_frame_time = now;

        if !self.is_panning && self.pan_velocity.length_squared() > 1.0 {
            self.pan(self.pan_velocity * dt);

            let friction = 5.0_f32;
            self.pan_velocity *= (-friction * dt).exp();

            if self.pan_velocity.length_squared() < 4.0 {
                self.pan_velocity = Vec2::ZERO;
            }

            return true;
        }

        false
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.renderer.resize(PhysicalSize::new(width, height));
    }

    pub fn pan(&mut self, delta: Vec2) {
        self.renderer.camera.offset.x -= delta.x / self.renderer.camera.zoom;
        self.renderer.camera.offset.y += delta.y / self.renderer.camera.zoom;
    }
    pub fn zoom(&mut self, cursor_pos: Vec2, delta: f32) {
        self.renderer.camera.zoom_at(cursor_pos, delta);
    }
}
