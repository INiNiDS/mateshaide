use crate::canvas::Canvas;
use crate::canvas::DrawTool;
use crate::graphics::Renderer;
use crate::shape::{ShapeError, ShapeRegistry};
use crate::dev::{TestMode, TestSpawner};
use std::sync::Arc;
use web_time::Instant;
use winit::dpi::PhysicalSize;
use winit::window::Window;

pub struct Engine {
    pub canvas: Canvas,
    pub shapes: ShapeRegistry,
    pub test_spawner: TestSpawner, // for test
    pub draw: DrawTool,
    renderer: Renderer,
    last_frame_time: Instant,
}

impl Engine {
    pub async fn new(window: Arc<Window>) -> Result<Self, EngineError> {
        let size = window.inner_size();
        let mut canvas = Canvas::new(size.width as f32, size.height as f32);
        let shapes: ShapeRegistry = ShapeRegistry::with_builtins()?;
        let renderer = Renderer::new(window, canvas.camera.uniform(), &shapes).await?;
        let actual = renderer.size();
        canvas.resize(actual.width as f32, actual.height as f32);

        Ok(Self {
            canvas,
            shapes,
            renderer,
            draw: Default::default(),
            last_frame_time: Instant::now(),
            test_spawner: TestSpawner::default(), // Guess what
        })
    }

    #[cfg(not(target_arch = "wasm32"))] // TODO: remove this when we have a proper test framework for wasm
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
        let swatch = if self.test_spawner.mode == Some(TestMode::Spawn) {
            self.test_spawner.preview_color()
        } else {
            None
        };

        self.renderer.redraw(
            &self.canvas.camera,
            &self.canvas.instances,
            &self.canvas.texts,
            &self.shapes,
            swatch,
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

    pub fn size(&self) -> PhysicalSize<u32> {
        self.renderer.size()
    }

    // Test section (will be removed later)

    pub fn toggle_test_spawn(&mut self) {
        let was_spawn = matches!(self.test_spawner.mode, Some(TestMode::Spawn));
        self.test_spawner.toggle_mode(TestMode::Spawn);

        if !was_spawn && self.test_spawner.current.is_none() {
            if let Some(kind) = self.shapes.kind("rect") {
                self.test_spawner.select("rect", kind);
            }
        }
    }

    pub fn select_test_shape(&mut self, name: &'static str) {
        if !matches!(self.test_spawner.mode, Some(TestMode::Spawn)) {
            tracing::warn!("Test spawn: not in spawn mode, cannot select shape");
            return;
        }

        if let Some(kind) = self.shapes.kind(name) {
            self.test_spawner.select(name, kind);
        } else {
            tracing::warn!("Test spawn: unknown shape kind '{name}'");
        }
    }

    pub fn spawn_test_shape(&mut self) {
        match self.test_spawner.mode {
            Some(TestMode::Spawn) => {
                self.test_spawner.spawn(&mut self.canvas);
            }
            Some(TestMode::Erase) => {
                if self.test_spawner.erase_at_cursor(&mut self.canvas) {
                    tracing::info!("Test spawn: erased shape at cursor");
                } else {
                    tracing::info!("Test spawn: no shape at cursor to erase");
                }
            }
            None => {
                tracing::warn!("Test spawn: not in spawn or erase mode, cannot spawn shape");
            }
        }
    }

    pub fn toggle_test_color_mode(&mut self) {
        self.test_spawner.toggle_color_mode();
    }

    pub fn next_test_picked_color(&mut self) {
        self.test_spawner.next_picked_color();
    }

    pub fn toggle_test_eraser(&mut self) {
        self.test_spawner.toggle_mode(TestMode::Erase);
    }

    pub fn erase_test_at_cursor(&mut self) {
        if self.test_spawner.erase_at_cursor(&mut self.canvas) {
            tracing::info!("Test spawn: erased shape at cursor");
        } else {
            tracing::info!("Test spawn: no shape at cursor to erase");
        }
    }

    pub fn undo(&mut self) -> bool {
        self.canvas.undo()
    }

    pub fn redo(&mut self) -> bool {
        self.canvas.redo()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error(transparent)]
    Shape(#[from] ShapeError),
    #[error(transparent)]
    Renderer(#[from] crate::graphics::RendererError),
}
