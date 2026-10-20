use crate::camera::Camera;
use crate::color::Color;
use glam::Vec2;
use glyphon::{
    Attrs, Buffer, Cache, Family, FontSystem, Metrics, PrepareError, Resolution, Shaping,
    SwashCache, TextArea, TextAtlas, TextBounds, TextRenderer, Viewport,
};
use wgpu::MultisampleState;

const MIN_SCREEN_FONT_PX: f32 = 4.0; // Too small...
const MAX_SCREEN_FONT_PX: f32 = 2000000000000000000.0; // Too big... Hmmm I'm thinking that doing nothing, because of is visible function...
const FONT_BYTES: &[u8] = include_bytes!("../assets/Roboto-Black.ttf");

#[derive(Debug, Clone)]
pub struct CanvasText {
    pub position: Vec2, // top left corner
    pub text: String,
    pub size: f32, // in world units
    pub color: Color,
}

struct LaidOutText {
    buffer: Buffer,
    left: f32,
    top: f32,
    scale: f32,
    color: Color,
}

pub struct TextLayer {
    font_system: FontSystem,
    swash_cache: SwashCache,
    cache: Cache,
    viewport: Viewport,
    atlas: TextAtlas,
    renderer: TextRenderer,
    format: wgpu::TextureFormat,
    laid_out: Vec<LaidOutText>,
}

impl TextLayer {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        let mut font_system = FontSystem::new();
        font_system.db_mut().load_font_data(FONT_BYTES.to_vec());

        let cache = Cache::new(device);
        let viewport = Viewport::new(device, &cache);
        let mut atlas = TextAtlas::new(device, queue, &cache, format);
        let renderer = TextRenderer::new(&mut atlas, device, MultisampleState::default(), None);

        Self {
            font_system,
            swash_cache: SwashCache::new(),
            cache,
            viewport,
            atlas,
            renderer,
            format,
            laid_out: Vec::new(),
        }
    }

    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        camera: &Camera,
        texts: &[CanvasText],
    ) {
        let width = camera.screen_size.x.max(1.0) as u32;
        let height = camera.screen_size.y.max(1.0) as u32;
        self.viewport.update(queue, Resolution { width, height });

        self.laid_out.clear();
        for t in texts {
            let screen_size = t.size * camera.zoom;
            if !(MIN_SCREEN_FONT_PX..=MAX_SCREEN_FONT_PX).contains(&screen_size) {
                continue;
            }

            let screen = camera.world_to_screen(t.position);
            if !is_visible(screen, screen_size, &t.text, camera.screen_size) {
                continue;
            }

            let metrics = Metrics::new(t.size, t.size * 1.25);
            let mut buffer = Buffer::new(&mut self.font_system, metrics);
            buffer.set_text(
                &t.text,
                &Attrs::new()
                    .family(Family::Name("Roboto").into())
                    .color(t.color.into()),
                Shaping::Advanced,
                None,
            );
            buffer.shape_until_scroll(&mut self.font_system, false);

            self.laid_out.push(LaidOutText {
                buffer,
                left: screen.x,
                top: screen.y,
                scale: camera.zoom,
                color: t.color,
            })
        }

        match self.upload(device, queue, width, height) {
            Ok(()) => {}
            Err(PrepareError::AtlasFull) => {
                self.atlas.trim();
                if self.upload(device, queue, width, height).is_err() {
                    tracing::warn!("txt atlas full, recreating");
                    self.reset_atlas(device, queue);
                    if let Err(e) = self.upload(device, queue, width, height) {
                        tracing::warn!("txt prepare failed after atlas reset: {e:?}");
                    }
                }
            }
        }
    }

    pub fn render<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        if let Err(e) = self.renderer.render(&self.atlas, &self.viewport, pass) {
            tracing::warn!("txt render failed: {e:?}");
        }
    }

    pub fn trim(&mut self) {
        self.atlas.trim();
    }

    fn upload(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        width: u32,
        height: u32,
    ) -> Result<(), PrepareError> {
        let areas = self.laid_out.iter().map(|t| TextArea {
            buffer: &t.buffer,
            left: t.left,
            top: t.top,
            scale: t.scale,
            bounds: TextBounds {
                left: 0,
                top: 0,
                right: width as i32,
                bottom: height as i32,
            },
            default_color: t.color.into(),
            custom_glyphs: &[],
        });

        self.renderer.prepare(
            device,
            queue,
            &mut self.font_system,
            &mut self.atlas,
            &self.viewport,
            areas,
            &mut self.swash_cache,
        )
    }

    fn reset_atlas(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        self.atlas = TextAtlas::new(device, queue, &self.cache, self.format);
        self.renderer =
            TextRenderer::new(&mut self.atlas, device, MultisampleState::default(), None);
    }
}

fn is_visible(screen: Vec2, px: f32, text: &str, screen_size: Vec2) -> bool {
    let approx_w = text.chars().count() as f32 * px * 0.7;
    let approx_h = px * 1.25;

    let right = screen.x + approx_w;
    let bottom = screen.y + approx_h;

    right > 0.0 && bottom > 0.0 && screen.x < screen_size.x && screen.y < screen_size.y
}
