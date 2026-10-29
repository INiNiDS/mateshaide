use super::TestMode;
use crate::engine::Engine;
use glam::Vec2;

// Works in debug, but not in release.
impl Engine {
    pub(crate) fn finish_test_interaction(&mut self) {
        self.test_spawner.finish_interaction(&mut self.canvas);
    }

    pub(crate) fn disable_test_tools(&mut self) {
        self.finish_test_interaction();
        self.test_spawner.mode = None;
        self.test_spawner.clear_selection();
    }

    fn toggle_test_mode(&mut self, mode: TestMode) {
        self.clear_plugin_tool();
        self.finish_test_interaction();
        self.draw.end(&mut self.canvas);
        self.draw.enabled = false;
        self.canvas.cancel_pan();
        self.test_spawner.toggle_mode(mode);
        if mode == TestMode::Spawn && self.test_spawner.current.is_none() {
            if let Some(kind) = self.shapes.kind("rect") {
                self.test_spawner.select("rect", kind);
            }
        }
    }

    pub fn toggle_test_spawn(&mut self) {
        self.toggle_test_mode(TestMode::Spawn);
    }

    pub fn toggle_test_selection(&mut self) {
        self.toggle_test_mode(TestMode::Select);
    }

    pub fn toggle_test_eraser(&mut self) {
        self.toggle_test_mode(TestMode::Erase);
    }

    pub fn select_test_shape(&mut self, name: &'static str) {
        if self.test_spawner.mode != Some(TestMode::Spawn) {
            return;
        }
        self.finish_test_interaction();
        if let Some(kind) = self.shapes.kind(name) {
            self.test_spawner.select(name, kind);
        }
    }

    pub fn spawn_test_shape(&mut self) {
        match self.test_spawner.mode {
            Some(TestMode::Spawn) => self.test_spawner.spawn(&mut self.canvas),
            Some(TestMode::Erase) => { self.erase_test_at_cursor(); }
            _ => {}
        }
    }

    pub fn erase_test_at_cursor(&mut self) {
        self.finish_test_interaction();
        self.test_spawner.erase_at_cursor(&mut self.canvas, &self.shapes);
    }

    pub fn delete_test_selection(&mut self) {
        self.test_spawner.delete_selected(&mut self.canvas);
    }

    pub fn escape_test_interaction(&mut self) {
        self.test_spawner.escape(&mut self.canvas);
    }

    pub fn set_test_spawn_size(&mut self, size: Vec2) {
        self.test_spawner.set_spawn_size(size);
    }

    pub fn scale_test_spawn_axes(&mut self, factor: Vec2) {
        if self.test_spawner.mode == Some(TestMode::Spawn) && !self.test_spawner.is_dragging() {
            self.test_spawner.scale_spawn_axes(factor);
        }
    }

    pub fn toggle_test_color_mode(&mut self) {
        self.test_spawner.toggle_color_mode();
    }

    pub fn next_test_picked_color(&mut self) {
        self.test_spawner.next_picked_color();
    }
}
