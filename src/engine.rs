use crate::canvas::Canvas;
use crate::renderer::Renderer;
use crate::shape::{ShapeError, ShapeRegistry};
use std::sync::Arc;
use std::time::Instant;
use winit::dpi::PhysicalSize;
use winit::window::Window;
use crate::test_spawn::TestSpawner; // For test

pub struct Engine {
    pub canvas: Canvas,
    pub shapes: ShapeRegistry,
    pub test_spawner: TestSpawner, // for test
    renderer: Renderer,
    last_frame_time: Instant,
}

impl Engine {
    pub async fn new(window: Arc<Window>) -> Result<Self, ShapeError> {
        let size = window.inner_size();
        let canvas = Canvas::new(size.width as f32, size.height as f32);
        let shapes: ShapeRegistry = ShapeRegistry::with_builtins()?;
        let renderer = Renderer::new(window, canvas.camera.uniform(), &shapes).await;

        Ok(Self {
            canvas,
            shapes,
            renderer,
            last_frame_time: Instant::now(),
            test_spawner: TestSpawner::default(), // Guess what
        })
    }

    pub fn define_shape<F>(&mut self, define: F) -> Result<u32, ShapeError>
    where
        F: FnOnce(&mut ShapeRegistry) -> Result<u32, ShapeError>,
    {
        let backup = self.shapes.clone();
        let kind = define(&mut self.shapes)?;

        if let Err(err) = self.renderer.validate_wgsl(&self.shapes.build_wgsl()) {
            self.shapes = backup;
            return Err(ShapeError::Gpu(err));
        }
        Ok(kind)
    }

    pub fn redraw(&mut self) {
        self.renderer.redraw(
            self.canvas.camera.uniform(),
            &self.canvas.instances,
            &self.shapes
        )
    }

    pub fn update(&mut self) -> bool {
        let now = Instant::now();
        let dt = (now - self.last_frame_time).as_secs_f32().min(0.1);
        self.last_frame_time = now;
        self.canvas.update(dt)
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.canvas.resize(width as f32, height as f32);
        self.renderer.resize(PhysicalSize::new(width, height))
    }

    pub fn reset_frame_time(&mut self) {
        self.last_frame_time = Instant::now();
    }

    // Test section (will be removed later)

    pub fn toggle_test_spawn(&mut self) {
        self.test_spawner.toggle();
        if self.test_spawner.active && self.test_spawner.current.is_none() {
            if let Some(kind) = self.shapes.kind("rect") {
                self.test_spawner.select("rect", kind);
            }
        }
    }

    pub fn select_test_shape(&mut self, name: &'static str) {
        if !self.test_spawner.active {
            tracing::warn!("Test spawn: not active, cannot select shape");
            return;
        }

        if let Some(kind) = self.shapes.kind(name) {
            self.test_spawner.select(name, kind);
        } else {
            tracing::warn!("Test spawn: unknown shape kind '{name}'");
        }
    }

    pub fn spawn_test_shape(&mut self) {
        if !self.test_spawner.active {
            tracing::warn!("Test spawn: not active, cannot spawn shape");
            return;
        }
        self.test_spawner.spawn(&mut self.canvas);
    }
}
