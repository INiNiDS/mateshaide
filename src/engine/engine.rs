use crate::canvas::Canvas;
use crate::canvas::DrawTool;
use crate::dev::{TestMode, TestSpawner};
use crate::export::png::CapturedPng;
use crate::export::ExportError;
use crate::export::bounds::{content_bounds, frame_camera, item_rects};
use crate::export::pdf::{PdfBuilder, RASTER_H, RASTER_W, page_tiles};
use crate::graphics::capture_overlay::{self, CaptureOverlayParams};
use crate::graphics::Renderer;
use crate::shape::{ShapeError, ShapeRegistry};
use crate::{RendererError, export};
use std::sync::Arc;
use web_time::Instant;
use winit::dpi::PhysicalSize;
use winit::window::Window;
use export::png::CaptureState;


const CONVERGE_DURATION_SECS: f32 = 0.35;
const FLASH_FADE_SECS: f32 = 0.15;
const PREVIEW_ENTER_SECS: f32 = 0.3;
const PREVIEW_FADE_SECS: f32 = 0.2;

pub struct Engine {
    pub canvas: Canvas,
    pub shapes: ShapeRegistry,
    pub test_spawner: TestSpawner, // for test
    pub draw: DrawTool,
    renderer: Renderer,
    last_frame_time: Instant,
    capture: CaptureState,
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
            draw: DrawTool::default(),
            last_frame_time: Instant::now(),
            test_spawner: TestSpawner::default(),
            capture: CaptureState::default(),
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
        let swatch = if self.capture.is_idle() && self.test_spawner.mode == Some(TestMode::Spawn) {
            self.test_spawner.preview_color()
        } else {
            None
        };

        let (sw, sh) = {
            let size = self.renderer.size();
            (size.width, size.height)
        };
        let capture_params = Self::capture_overlay_params(&self.capture, sw, sh);

        self.renderer.redraw(
            &self.canvas.camera,
            &self.canvas.instances,
            &self.canvas.texts,
            &self.shapes,
            swatch,
            capture_params,
        );
    }

    pub fn update(&mut self) -> bool {
        let now = Instant::now();
        let dt = (now - self.last_frame_time).as_secs_f32().min(0.1);
        self.last_frame_time = now;

        let pan_gliding = self.canvas.update(dt);
        let capture_active = self.update_capture();

        pan_gliding || capture_active
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.canvas.resize(width as f32, height as f32);
        self.renderer.resize(PhysicalSize::new(width, height));
    }

    pub fn reset_frame_time(&mut self) {
        self.last_frame_time = Instant::now();
    }

    pub fn size(&self) -> PhysicalSize<u32> {
        self.renderer.size()
    }

    pub fn begin_png_capture(&mut self) {
        match self.capture_frame() {
            Ok(image) => {
                self.capture = CaptureState::Converge {
                    started: Instant::now(),
                    image,
                };
            }
            Err(ExportError::Empty) => {
                tracing::warn!("nothing to capture: canvas is empty");
            }
            Err(e) => {
                tracing::error!("PNG capture failed: {e}");
            }
        }
    }

    fn capture_frame(&mut self) -> Result<CapturedPng, ExportError> {
        let rects = item_rects(&self.canvas.instances, &self.canvas.texts);
        let frame = content_bounds(&rects)
            .ok_or(ExportError::Empty)?
            .padded(export::PADDING);
        let (w, h) = export::png::pixel_size(frame);
        let camera = frame_camera(frame, w, h);

        let rgba = self
            .renderer
            .export_rgba(
                &camera,
                &self.canvas.instances,
                &self.canvas.texts,
                &self.shapes,
                w,
                h,
            )?;

        let png = export::png::encode(&rgba, w, h)?;

        Ok(CapturedPng {
            png,
            rgba,
            width: w,
            height: h,
        })
    }

    pub fn is_capturing(&self) -> bool {
        !self.capture.is_idle()
    }

    pub fn is_in_capture_preview(&self) -> bool {
        self.capture.is_preview()
    }

    pub fn save_capture(&mut self) {
        if let CaptureState::Preview { image, .. } = &self.capture {
            export::save_capture(&image.png);
        }
    }

    pub fn copy_capture(&mut self) {
        if let CaptureState::Preview { image, .. } = &self.capture {
            export::copy_capture(&image.rgba, image.width, image.height);
        }
    }

    pub fn dismiss_capture(&mut self) {
        self.capture = CaptureState::Idle;
    }

    fn update_capture(&mut self) -> bool {
        let now = Instant::now();

        let transition = match &self.capture {
            CaptureState::Idle => return false,
            CaptureState::Preview { shown_at, .. } => {
                return (now - *shown_at).as_secs_f32() < PREVIEW_ENTER_SECS;
            }
            CaptureState::Converge { started, .. } => {
                (now - *started).as_secs_f32() >= CONVERGE_DURATION_SECS
            }
        };

        if transition {
            if let CaptureState::Converge { image, .. } =
                std::mem::replace(&mut self.capture, CaptureState::Idle)
            {
                self.capture = CaptureState::Preview {
                    image,
                    shown_at: now,
                };
            }
        }
        true
    }

    fn capture_overlay_params(
        capture: &CaptureState,
        screen_w: u32,
        screen_h: u32,
    ) -> CaptureOverlayParams<'_> {
        let now = Instant::now();
        match capture {
            CaptureState::Idle => CaptureOverlayParams::None,

            CaptureState::Converge { started, .. } => {
                let elapsed = (now - *started).as_secs_f32();
                let progress = (elapsed / CONVERGE_DURATION_SECS).clamp(0.0, 1.0);
                CaptureOverlayParams::Converge { progress }
            }

            CaptureState::Preview { image, shown_at } => {
                let elapsed = (now - *shown_at).as_secs_f32();

                let flash_alpha = (1.0 - elapsed / FLASH_FADE_SECS).clamp(0.0, 1.0);
                let enter = (elapsed / PREVIEW_ENTER_SECS).clamp(0.0, 1.0);
                let opacity = (elapsed / PREVIEW_FADE_SECS).clamp(0.0, 1.0);

                let base_rect = capture_overlay::compute_preview_rect(
                    image.width,
                    image.height,
                    screen_w,
                    screen_h,
                );

                let s = 0.85 + 0.15 * enter;
                let rect = [
                    base_rect[0] * s,
                    base_rect[1] * s,
                    base_rect[2] * s,
                    base_rect[3] * s,
                ];

                CaptureOverlayParams::Preview {
                    image,
                    flash_alpha,
                    rect,
                    opacity,
                }
            }
        }
    }

    pub fn toggle_test_spawn(&mut self) {
        let was_spawn = matches!(self.test_spawner.mode, Some(TestMode::Spawn));
        self.test_spawner.toggle_mode(TestMode::Spawn);

        if !was_spawn
            && self.test_spawner.current.is_none()
            && let Some(kind) = self.shapes.kind("rect")
        {
            self.test_spawner.select("rect", kind);
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
            Some(TestMode::Spawn) => self.test_spawner.spawn(&mut self.canvas),
            Some(TestMode::Erase) => {
                self.test_spawner.erase_at_cursor(&mut self.canvas);
            }
            None => {
                tracing::warn!("Test spawn: not in spawn or erase mode");
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
        self.test_spawner.erase_at_cursor(&mut self.canvas);
    }

    pub fn undo(&mut self) -> bool {
        self.canvas.undo()
    }

    pub fn redo(&mut self) -> bool {
        self.canvas.redo()
    }

    pub fn export_pdf(&mut self) -> Result<Vec<u8>, ExportError> {
        let rects = item_rects(&self.canvas.instances, &self.canvas.texts);
        let content = content_bounds(&rects).ok_or(ExportError::Empty)?;
        let tiles = page_tiles(content, &rects)?;

        let (w, h) = (RASTER_W, RASTER_H);
        let mut builder = PdfBuilder::new();

        for tile in tiles {
            let camera = frame_camera(tile, w, h);
            let rgba = self.renderer.export_rgba(
                &camera,
                &self.canvas.instances,
                &self.canvas.texts,
                &self.shapes,
                w,
                h,
            )?;
            builder.add_page(rgba)?;
        }
        builder.finish()
    }

    pub fn export_png(&mut self) -> Result<Vec<u8>, ExportError> {
        let rects = item_rects(&self.canvas.instances, &self.canvas.texts);
        let frame = content_bounds(&rects)
            .ok_or(ExportError::Empty)?
            .padded(export::PADDING);

        let (w, h) = export::png::pixel_size(frame);
        let camera = frame_camera(frame, w, h);

        let rgba = self.renderer.export_rgba(
            &camera,
            &self.canvas.instances,
            &self.canvas.texts,
            &self.shapes,
            w,
            h,
        )?;

        export::png::encode(&rgba, w, h)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error(transparent)]
    Shape(#[from] ShapeError),
    #[error(transparent)]
    Renderer(#[from] RendererError),
}
