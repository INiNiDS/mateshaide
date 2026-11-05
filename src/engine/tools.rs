use crate::Engine;
use crate::canvas::eraser::EraseOutcome;
use crate::canvas::hit;
use crate::{ShapeInstance, palette};
use glam::Vec2;
use winit::window::CursorIcon;

const HIGHLIGHT_PX: f32 = 1.5;

impl Engine {
    pub fn set_eraser(&mut self, enabled: bool) {
        if enabled && !self.eraser.enabled {
            self.clear_plugin_tool();
            self.draw.end(&mut self.canvas);
            self.draw.enabled = false;
            self.plugin_page.draw_enabled = false;
            #[cfg(debug_assertions)]
            self.disable_test_tools();
            self.canvas.cancel_pan();
        }
        self.eraser.set_enabled(&mut self.canvas, enabled);
        self.sync_plugin_focus();
    }

    pub fn toggle_eraser(&mut self) {
        self.set_eraser(!self.eraser.enabled);
    }

    pub fn escape(&mut self) {
        if self.eraser.enabled {
            self.set_eraser(false);
        } else if self.plugins.selected.is_some() && !self.plugins.wants_text_input() {
            self.clear_plugin_tool();
        }
        #[cfg(debug_assertions)]
        self.escape_test_interaction();
    }

    fn apply_erase(&mut self, out: EraseOutcome) -> bool {
        if out.nodes {
            self.plugins.document_changed(&mut self.canvas);
        }
        out.redraw
    }

    pub fn eraser_pressed(&mut self, screen: Vec2) -> bool {
        let out = self.eraser.press(&mut self.canvas, &self.shapes, screen);
        self.apply_erase(out)
    }

    pub fn eraser_moved(&mut self, screen: Vec2) -> bool {
        let out = self.eraser.moved(&mut self.canvas, &self.shapes, screen);
        self.apply_erase(out)
    }

    pub fn eraser_released(&mut self) {
        self.eraser.release(&mut self.canvas);
    }

    #[must_use]
    pub fn cursor_icon(&self) -> CursorIcon {
        if self.eraser.enabled {
            CursorIcon::Crosshair
        } else if self.text_tool_selected() {
            CursorIcon::Text
        } else {
            CursorIcon::Default
        }
    }

    pub(crate) fn eraser_highlight(&self) -> Vec<ShapeInstance> {
        let Some(r) = self
            .eraser
            .hover
            .filter(|_| self.eraser.enabled)
            .and_then(|h| hit::hit_bounds(&self.canvas, h))
        else {
            return Vec::new();
        };
        let t = HIGHLIGHT_PX / self.canvas.camera.zoom;
        let size = r.max - r.min;
        let color = palette::RED_500;
        vec![
            ShapeInstance::rect(r.min.x - t, r.min.y - t, size.x + 2.0 * t, t, color),
            ShapeInstance::rect(r.min.x - t, r.max.y, size.x + 2.0 * t, t, color),
            ShapeInstance::rect(r.min.x - t, r.min.y, t, size.y, color),
            ShapeInstance::rect(r.max.x, r.min.y, t, size.y, color),
        ]
    }
}
