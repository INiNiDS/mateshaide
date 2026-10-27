use crate::canvas::Canvas;
use crate::canvas::DrawTool;
use crate::dev::{TestMode, TestSpawner};
use crate::export::bounds::{Rect, content_bounds, frame_camera, item_rects};
use crate::export::pdf::{PdfBuilder, RASTER_H, RASTER_W, page_tiles};
use crate::export::png::{CaptureState, CapturedPng};
use crate::graphics::capture_overlay::{
    self, ButtonAction, ButtonRect, CaptureOverlayParams, CardAnim, CardLayout, REST_ANGLE,
};
use crate::graphics::{PendingReadback, Renderer, RendererError};
use crate::shape::{ShapeError, ShapeRegistry};
use crate::{CanvasText, Color, export};
use glam::Vec2;
use std::collections::VecDeque;
use std::sync::Arc;
use web_time::Instant;
use winit::dpi::PhysicalSize;
use winit::window::Window;

const CONVERGE_SECS: f32 = 0.45;
const FLASH_FADE_SECS: f32 = 0.18;
const PREVIEW_ANIM_SECS: f32 = 3.0;
const DROP_SECS: f32 = 0.75;
const DEVELOP_DELAY: f32 = 0.35;
const DEVELOP_SECS: f32 = 2.4;
const BUTTONS_DELAY: f32 = 0.7;
const BUTTONS_FADE: f32 = 0.25;

struct PdfJob {
    tiles: VecDeque<Rect>,
    builder: PdfBuilder,
    inflight: Option<PendingReadback>,
}

pub struct Engine {
    pub canvas: Canvas,
    pub shapes: ShapeRegistry,
    pub test_spawner: TestSpawner,
    pub draw: DrawTool,
    pub capture_cursor: Vec2,
    renderer: Renderer,
    last_frame_time: Instant,
    capture: CaptureState,
    pending_png: Option<PendingReadback>,
    pdf_job: Option<PdfJob>,
}

impl Engine {
    pub async fn new(window: Arc<Window>) -> Result<Self, EngineError> {
        let size = window.inner_size();
        let mut canvas = Canvas::new(size.width as f32, size.height as f32);
        let shapes = ShapeRegistry::with_builtins()?;
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
            capture_cursor: Vec2::ZERO,
            capture: CaptureState::default(),
            pending_png: None,
            pdf_job: None,
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
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
            let s = self.renderer.size();
            (s.width, s.height)
        };

        let overlay_texts = match self.capture_layout(sw, sh) {
            Some(l) => {
                let b = capture_overlay::compute_button_rects(&l, sw, sh);
                self.build_button_texts(&b, self.button_opacity())
            }
            None => Vec::new(),
        };
        let capture_params = Self::capture_params(&self.capture, sw, sh);

        self.renderer.redraw(
            &self.canvas.camera,
            &self.canvas.instances,
            &self.canvas.texts,
            &self.shapes,
            swatch,
            capture_params,
            &overlay_texts,
        );
    }

    pub fn update(&mut self) -> bool {
        let now = Instant::now();
        let dt = (now - self.last_frame_time).as_secs_f32().min(0.1);
        self.last_frame_time = now;

        let pan = self.canvas.update(dt);
        let png = self.poll_pending_png();
        let pdf = self.poll_pdf();
        let cap = self.update_capture();
        pan || png || pdf || cap
    }

    pub fn resize(&mut self, w: u32, h: u32) {
        self.canvas.resize(w as f32, h as f32);
        self.renderer.resize(PhysicalSize::new(w, h));
    }

    pub fn reset_frame_time(&mut self) {
        self.last_frame_time = Instant::now();
    }

    pub fn size(&self) -> PhysicalSize<u32> {
        self.renderer.size()
    }

    fn export_frame(&self) -> Option<Rect> {
        let rects = item_rects(&self.canvas.instances, &self.canvas.texts);
        Some(content_bounds(&rects)?.padded(export::PADDING))
    }

    pub fn begin_png_capture(&mut self) {
        if self.pending_png.is_some() {
            return;
        }
        let Some(frame) = self.export_frame() else {
            tracing::warn!("canvas is empty");
            return;
        };
        let (w, h) = export::png::pixel_size(frame);
        let camera = frame_camera(frame, w, h);

        match self.renderer.start_export(
            &camera,
            &self.canvas.instances,
            &self.canvas.texts,
            &self.shapes,
            w,
            h,
        ) {
            Ok(rb) => self.pending_png = Some(rb),
            Err(e) => tracing::error!("capture failed: {e}"),
        }
    }

    fn poll_pending_png(&mut self) -> bool {
        let Some(rb) = self.pending_png.take() else {
            return false;
        };
        let (w, h) = (rb.width, rb.height);

        match self.renderer.try_finish_export(&rb) {
            None => {
                self.pending_png = Some(rb);
                true
            }
            Some(Ok(rgba)) => {
                match export::png::encode(&rgba, w, h) {
                    Ok(png) => {
                        self.capture = CaptureState::Converge {
                            started: Instant::now(),
                            image: CapturedPng {
                                png,
                                rgba,
                                width: w,
                                height: h,
                            },
                        };
                    }
                    Err(e) => tracing::error!("capture encode failed: {e}"),
                }
                true
            }
            Some(Err(e)) => {
                tracing::error!("capture failed: {e}");
                true
            }
        }
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

    pub fn check_button_click(&mut self, cursor: Vec2) -> Option<ButtonAction> {
        if !self.capture.is_preview() {
            return None;
        }
        let (sw, sh) = {
            let s = self.renderer.size();
            (s.width, s.height)
        };
        let layout = self.capture_layout(sw, sh)?;
        let buttons = capture_overlay::compute_button_rects(&layout, sw, sh);

        for (i, btn) in buttons.iter().enumerate() {
            let [l, t, r, b] = btn.screen;
            if cursor.x >= l && cursor.x <= r && cursor.y >= t && cursor.y <= b {
                return Some(if i == 0 {
                    ButtonAction::Save
                } else {
                    ButtonAction::Copy
                });
            }
        }
        None
    }

    fn update_capture(&mut self) -> bool {
        let now = Instant::now();
        let transition = match &self.capture {
            CaptureState::Idle => return false,
            CaptureState::Preview { shown_at, .. } => {
                return (now - *shown_at).as_secs_f32() < PREVIEW_ANIM_SECS;
            }
            CaptureState::Converge { started, .. } => {
                (now - *started).as_secs_f32() >= CONVERGE_SECS
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

    fn capture_layout(&mut self, sw: u32, sh: u32) -> Option<CardLayout> {
        match &self.capture {
            CaptureState::Preview { image, .. } | CaptureState::Converge { image, .. } => Some(
                capture_overlay::compute_card_layout(image.width, image.height, sw, sh),
            ),
            CaptureState::Idle => None,
        }
    }

    fn buttons_opacity_at(t: f32) -> f32 {
        ((t - BUTTONS_DELAY) / BUTTONS_FADE).clamp(0.0, 1.0)
    }

    fn button_opacity(&self) -> f32 {
        match &self.capture {
            CaptureState::Preview { shown_at, .. } => {
                Self::buttons_opacity_at((Instant::now() - *shown_at).as_secs_f32())
            }
            _ => 0.0,
        }
    }

    fn card_anim(t: f32, l: &CardLayout, sh: f32) -> CardAnim {
        let x = (t / DROP_SECS).clamp(0.0, 1.0);
        let c1 = 1.70158;
        let c3 = c1 + 1.0;
        let u = x - 1.0;

        let p = 1.0 + c3 * u.powi(3) + c1 * u.powi(2);
        let start = sh * 0.5 + l.card.y * 0.5 + 40.0;

        CardAnim {
            offset: Vec2::new(0.0, -(1.0 - p) * start),
            angle: REST_ANGLE + (0.10 - REST_ANGLE) * (1.0 - p),
            scale: 0.92 + 0.08 * p.min(1.0),
            opacity: (t / 0.06).clamp(0.0, 1.0),
            develop: ((t - DEVELOP_DELAY) / DEVELOP_SECS).clamp(0.0, 1.0),
            shadow: p.clamp(0.0, 1.0),
            dim: (t / 0.35).clamp(0.0, 1.0) * 0.5,
        }
    }

    fn capture_params(capture: &CaptureState, sw: u32, sh: u32) -> CaptureOverlayParams<'_> {
        let now = Instant::now();
        match capture {
            CaptureState::Idle => CaptureOverlayParams::None,
            CaptureState::Converge { started, .. } => {
                let elapsed = (now - *started).as_secs_f32();
                let progress = (elapsed / CONVERGE_SECS).clamp(0.0, 1.0);
                CaptureOverlayParams::Converge { progress }
            }
            CaptureState::Preview { shown_at, image } => {
                let elapsed = (now - *shown_at).as_secs_f32();
                let flash = (1.0 - elapsed / FLASH_FADE_SECS).clamp(0.0, 1.0);
                let layout =
                    capture_overlay::compute_card_layout(image.width, image.height, sw, sh);

                CaptureOverlayParams::Preview {
                    image,
                    flash_alpha: flash,
                    anim: Self::card_anim(elapsed, &layout, sh as f32),
                    buttons: Box::from(capture_overlay::compute_button_rects(&layout, sw, sh)),
                    buttons_opacity: Self::buttons_opacity_at(elapsed),
                    layout,
                }
            }
        }
    }

    // TODO: Remove "Magic Numbers"
    fn build_button_texts(&self, buttons: &[ButtonRect; 2], opacity: f32) -> Vec<CanvasText> {
        if opacity < 0.01 {
            return Vec::new();
        }
        buttons
            .iter()
            .map(|btn| {
                let cx = (btn.screen[0] + btn.screen[2]) * 0.5;
                let cy = (btn.screen[1] + btn.screen[3]) * 0.5;
                let sz = 15.0 / self.canvas.camera.zoom;
                let est_w = btn.label.len() as f32 * sz * 0.32;
                let world = self
                    .canvas
                    .camera
                    .screen_to_world(Vec2::new(cx - est_w, cy - sz * 0.55));
                CanvasText {
                    position: world,
                    text: btn.label.to_string(),
                    size: sz,
                    color: Color::WHITE.with_alpha(opacity),
                }
            })
            .collect()
    }

    pub fn begin_pdf_export(&mut self) {
        if self.pdf_job.is_some() {
            return;
        }
        let rects = item_rects(&self.canvas.instances, &self.canvas.texts);
        let Some(content) = content_bounds(&rects) else {
            tracing::warn!("canvas is empty");
            return;
        };
        match page_tiles(content, &rects) {
            Ok(tiles) => {
                self.pdf_job = Some(PdfJob {
                    tiles: tiles.into(),
                    builder: PdfBuilder::new(),
                    inflight: None,
                });
            }
            Err(e) => tracing::error!("pdf failed: {e}"),
        }
    }

    fn poll_pdf(&mut self) -> bool {
        let Some(mut job) = self.pdf_job.take() else {
            return false;
        };

        if let Some(rb) = job.inflight.take() {
            match self.renderer.try_finish_export(&rb) {
                None => {
                    job.inflight = Some(rb);
                    self.pdf_job = Some(job);
                    return true;
                }
                Some(Ok(rgba)) => {
                    if let Err(e) = job.builder.add_page(rgba) {
                        tracing::error!("pdf page failed: {e}");
                        return true;
                    }
                }
                Some(Err(e)) => {
                    tracing::error!("pdf readback failed: {e}");
                    return true;
                }
            }
        }

        if let Some(tile) = job.tiles.pop_front() {
            let camera = frame_camera(tile, RASTER_W, RASTER_H);
            match self.renderer.start_export(
                &camera,
                &self.canvas.instances,
                &self.canvas.texts,
                &self.shapes,
                RASTER_W,
                RASTER_H,
            ) {
                Ok(rb) => job.inflight = Some(rb),
                Err(e) => {
                    tracing::error!("pdf page start failed: {e}");
                    return true;
                }
            }
            self.pdf_job = Some(job);
            return true;
        }

        match job.builder.finish() {
            Ok(bytes) => {
                let stamp = web_time::SystemTime::now()
                    .duration_since(web_time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                export::save::save_bytes(
                    &bytes,
                    "application/pdf",
                    &format!("mateshaide-{stamp}.pdf"),
                );
            }
            Err(e) => tracing::error!("pdf finish failed: {e}"),
        }
        true
    }

    pub fn toggle_test_spawn(&mut self) {
        let was = matches!(self.test_spawner.mode, Some(TestMode::Spawn));
        self.test_spawner.toggle_mode(TestMode::Spawn);
        if !was
            && self.test_spawner.current.is_none()
            && let Some(k) = self.shapes.kind("rect")
        {
            self.test_spawner.select("rect", k);
        }
    }

    pub fn select_test_shape(&mut self, name: &'static str) {
        if !matches!(self.test_spawner.mode, Some(TestMode::Spawn)) {
            tracing::warn!("not in spawn mode");
            return;
        }
        if let Some(k) = self.shapes.kind(name) {
            self.test_spawner.select(name, k);
        }
    }

    pub fn spawn_test_shape(&mut self) {
        match self.test_spawner.mode {
            Some(TestMode::Spawn) => self.test_spawner.spawn(&mut self.canvas),
            Some(TestMode::Erase) => {
                self.test_spawner.erase_at_cursor(&mut self.canvas);
            }
            None => {}
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
}

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error(transparent)]
    Shape(#[from] ShapeError),
    #[error(transparent)]
    Renderer(#[from] RendererError),
}
