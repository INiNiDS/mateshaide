use glam::Vec2;
use crate::canvas::Canvas;
use crate::color::Color;
use crate::palette;

pub struct DrawTool {
    pub enabled: bool,
    pub width: f32,
    pub color: Color
}

impl Default for DrawTool {
    fn default() -> DrawTool {
        Self {
            enabled: false,
            width: 4.0,
            color: palette::GRAY_900
        }
    }
}

impl DrawTool {
    pub fn toggle(&mut self) {
        self.enabled = !self.enabled;
        tracing::info!("draw mode: {}", self.enabled);
    }

    pub fn begin(&self, canvas: &mut Canvas, screen: Vec2) {
        if !self.enabled {
            return;
        }
        canvas.begin_stroke();
        self.point(canvas, screen);
    }

    pub fn point(&self, canvas: &mut Canvas, screen: Vec2) {
        let world = canvas.camera.screen_to_world(screen);
        canvas.extend_stroke(world, self.width, self.color);
    }

    pub fn end(&self, canvas: &mut Canvas) {
        canvas.end_stroke();
    }
}