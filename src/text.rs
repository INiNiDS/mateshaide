use crate::camera::Camera;
use crate::color::Color;
use glam::Vec2;
use glyphon::{
    Attrs, Buffer, Cache, Family, FontSystem, Metrics, Resolution, Shaping, SwashCache, TextArea,
    TextAtlas, TextBounds, TextRenderer, Viewport,
};
use wgpu::MultisampleState;

const FONT_BYTES: &[u8] = include_bytes!("../assets/Roboto-Black.ttf");

#[derive(Debug, Clone)]
pub struct CanvasText {
    pub position: Vec2, // top left corner
    pub text: String,
    pub size: f32, // in world units
    pub color: Color,
}

pub struct TextLayer {
    font_system: FontSystem,
    swash_cache: SwashCache,
    viewport: Viewport,
    atlas: TextAtlas,
    renderer: TextRenderer,
    buffers: Vec<Buffer>,
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
            viewport,
            atlas,
            renderer,
            buffers: Vec::new(),
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

        self.buffers.clear();
        for t in texts {
            let metrics = Metrics::new(t.size, t.size * 1.25);
            let mut buffer = Buffer::new(&mut self.font_system, metrics);
            buffer.set_text(
                &t.text,
                &Attrs::new().family(Family::SansSerif),
                Shaping::Advanced,
                None,
            );
            buffer.shape_until_scroll(&mut self.font_system, false);
            self.buffers.push(buffer);
        }

        let areas = texts.iter().zip(self.buffers.iter()).map(|(t, b)| {
            let screen = camera.world_to_screen(t.position);
            TextArea {
                buffer: b,
                left: screen.x,
                top: screen.y,
                scale: camera.zoom,
                bounds: TextBounds {
                    left: 0,
                    top: 0,
                    right: width as i32,
                    bottom: height as i32,
                },
                default_color: t.color.into(),
                custom_glyphs: &[],
            }
        });

        if let Err(e) = self.renderer.prepare(
            device,
            queue,
            &mut self.font_system,
            &mut self.atlas,
            &self.viewport,
            areas,
            &mut self.swash_cache
        ) {
            tracing::warn!("txt prepare failed: {e:?}");
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
}
