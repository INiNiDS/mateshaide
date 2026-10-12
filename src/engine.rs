use crate::canvas::Canvas;
use crate::renderer::Renderer;
use std::sync::Arc;
use std::time::Instant;
use winit::dpi::PhysicalSize;
use winit::window::Window;

// Struct for all engine components.
pub struct Engine {
    pub canvas: Canvas,
    renderer: Renderer,
    last_frame_time: Instant,
}

impl Engine {
    pub async fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();
        let canvas = Canvas::new(size.width as f32, size.height as f32);
        let uniform = canvas.camera.uniform();
        Self {
            canvas,
            renderer: Renderer::new(window, uniform).await,
            last_frame_time: Instant::now(),
        }
    }

    pub fn redraw(&mut self) {
        self.renderer.redraw(self.canvas.camera.uniform());
    }

    pub fn update(&mut self) -> bool {
        let now = Instant::now();
        let dt = (now - self.last_frame_time).as_secs_f32().min(0.1);
        self.last_frame_time = now;
        self.canvas.update(dt)
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.canvas.resize(width as f32, height as f32);
        self.renderer.resize(PhysicalSize::new(width, height));
    }
    
    pub fn reset_frame_time(&mut self) {
        self.last_frame_time = Instant::now();
    }
}
