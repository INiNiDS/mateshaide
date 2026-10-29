use crate::{
    Engine,
    plugins::{self, Scene, UiAction, UiFrame},
};
use glam::Vec2;
use serde_json::{Value, json};
use winit::{
    event::{ElementState, Ime, MouseButton, MouseScrollDelta, WindowEvent},
    keyboard::{Key, KeyCode, ModifiersState, PhysicalKey},
};

impl Engine {
    pub fn clear_plugin_tool(&mut self) {
        self.plugins.blur(&mut self.canvas);
        self.plugins.selected = None;
        self.plugin_pointer_captured = false;
        self.plugin_ime_composing = false;
        plugins::platform::editor_focus(false, 0.0, 0.0);
    }
    pub fn sync_plugin_focus(&self) {
        let enabled =
            !self.plugin_page.open && !self.is_capturing() && self.plugins.wants_text_input();
        plugins::platform::editor_focus(enabled, self.capture_cursor.x, self.capture_cursor.y);
    }
    pub(crate) fn plugin_scene(&mut self, overlay: bool) -> Scene {
        let show_overlay = overlay && !self.plugin_page.open && !self.is_capturing();
        let mut scene = self.plugins.project(
            &self.canvas.plugin_nodes,
            show_overlay,
            self.canvas.camera.zoom,
        );
        scene
            .shapes
            .splice(0..0, self.canvas.instances.iter().copied());
        scene.texts.splice(0..0, self.canvas.texts.iter().cloned());
        scene
    }
    pub(crate) fn plugin_frame(&self) -> UiFrame {
        if self.is_capturing() {
            UiFrame::default()
        } else {
            self.plugins
                .ui_frame(&self.plugin_page, &self.canvas.camera)
        }
    }
    fn plugin_action(&mut self, action: UiAction) {
        match action {
            UiAction::TogglePage => {
                self.plugins.blur(&mut self.canvas);
                self.draw.end(&mut self.canvas);
                self.canvas.cancel_pan();
                self.plugin_pointer_captured = false;
                self.plugin_page.open = !self.plugin_page.open;
                self.plugins.confirmation = None;
                self.plugin_page.scroll = 0.0;
            }
            UiAction::SelectPlugin(id) => self.plugin_page.selected = Some(id),
            UiAction::TogglePlugin(id) => {
                self.plugin_page.scroll = 0.0;
                if self.plugins.active(&id) {
                    self.plugins.request_disable(&id);
                } else {
                    self.plugins.request_enable(&id);
                }
            }
            UiAction::Confirm => self.plugins.confirm(),
            UiAction::Cancel => self.plugins.confirmation = None,
            UiAction::Rescan => {
                self.clear_plugin_tool();
                self.plugins.rescan();
            }
            UiAction::Repository(url) => plugins::platform::open_repository(&url),
            UiAction::Tool(p, t) => {
                self.draw.end(&mut self.canvas);
                self.draw.enabled = false;
                #[cfg(debug_assertions)]
                self.disable_test_tools();
                self.plugin_pointer_captured = false;
                self.plugins.select_tool(&p, &t, &mut self.canvas);
            }
            UiAction::Command(p, c) => {
                self.plugins
                    .dispatch(&p, json!({"name":"command","id":c}), &mut self.canvas)
            }
            UiAction::Save => match self.canvas.document_bytes() {
                Ok(b) => plugins::platform::save_document(&b),
                Err(e) => self.plugins.notice = e.to_string(),
            },
            UiAction::Open => plugins::platform::request_open(),
            UiAction::Import => plugins::platform::request_import(),
        }
        self.sync_plugin_focus();
    }
    fn send_plugin_event(&mut self, event: Value) -> bool {
        if let Some((id, _)) = self.plugins.selected.clone() {
            self.plugins.dispatch(&id, event, &mut self.canvas);
            self.sync_plugin_focus();
            true
        } else {
            false
        }
    }
    // TODO
    pub(crate) fn handle_plugin_event(
        &mut self,
        event: &WindowEvent,
        cursor: Vec2,
        mods: ModifiersState,
    ) -> bool {
        let ctrl = mods.control_key() || mods.super_key();
        let shift = mods.shift_key();
        // CursorMoved carries the current position, not InputManager's previous position.
        let position = match event {
            WindowEvent::CursorMoved { position, .. } => {
                Vec2::new(position.x as f32, position.y as f32)
            }
            _ => cursor,
        };
        self.capture_cursor = position;
        if let WindowEvent::KeyboardInput { event, .. } = event {
            if event.state == ElementState::Pressed {
                if let PhysicalKey::Code(code) = event.physical_key {
                    if code == KeyCode::KeyP && ctrl && shift {
                        self.plugin_action(UiAction::TogglePage);
                        return true;
                    }
                    if code == KeyCode::KeyS && ctrl {
                        self.plugin_action(UiAction::Save);
                        return true;
                    }
                    if code == KeyCode::KeyO && ctrl {
                        self.plugin_action(UiAction::Open);
                        return true;
                    }
                    if code == KeyCode::F5 && self.plugin_page.open {
                        self.plugin_action(UiAction::Rescan);
                        return true;
                    }
                    if code == KeyCode::Escape && self.plugin_page.open {
                        if self.plugins.confirmation.is_some() {
                            self.plugins.confirmation = None;
                        } else {
                            self.plugin_action(UiAction::TogglePage);
                        }
                        return true;
                    }
                    // Preserve global undo/redo, export, draw and camera shortcuts.
                    if ctrl
                        && matches!(
                            code,
                            KeyCode::KeyZ
                                | KeyCode::KeyY
                                | KeyCode::KeyE
                                | KeyCode::KeyP
                                | KeyCode::Digit9
                                | KeyCode::Equal
                                | KeyCode::Minus
                                | KeyCode::Digit0
                                | KeyCode::NumpadAdd
                                | KeyCode::NumpadSubtract
                        )
                    {
                        return self.plugin_page.open;
                    }
                    if ctrl && code == KeyCode::KeyX && !self.plugins.wants_text_input() {
                        return self.plugin_page.open;
                    }
                }
            }
        }
        if let WindowEvent::MouseInput {
            button: MouseButton::Left,
            state: ElementState::Pressed,
            ..
        } = event
        {
            if let Some(action) = self.plugin_frame().hit(position) {
                self.plugin_action(action);
                return true;
            }
        }
        if self.plugin_page.open {
            if let WindowEvent::MouseWheel { delta, .. } = event {
                let d = match delta {
                    MouseScrollDelta::LineDelta(_, y) => *y * 40.0,
                    MouseScrollDelta::PixelDelta(p) => p.y as f32,
                };
                self.plugin_page.scroll = (self.plugin_page.scroll - d).clamp(0.0, 12000.0);
            }
            return true;
        }
        match event {
            WindowEvent::Focused(false) => {
                self.plugins.blur(&mut self.canvas);
                self.plugin_pointer_captured = false;
                self.plugin_ime_composing = false;
                self.sync_plugin_focus();
                false
            }
            WindowEvent::Ime(Ime::Enabled) => false,
            WindowEvent::Ime(Ime::Preedit(text, _)) => {
                self.plugin_ime_composing = !text.is_empty();
                self.send_plugin_event(json!({"name":"ime.preedit","text":text}))
            }
            WindowEvent::Ime(Ime::Commit(text)) => {
                self.plugin_ime_composing = false;
                self.send_plugin_event(json!({"name":"text","text":text}))
            }
            WindowEvent::Ime(Ime::Disabled) => {
                self.plugin_ime_composing = false;
                self.send_plugin_event(json!({"name":"ime.preedit","text":""}))
            }
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                ..
            } if self.plugins.selected.is_some() => {
                let phase = if *state == ElementState::Pressed {
                    "down"
                } else {
                    "up"
                };
                self.plugin_pointer_captured = *state == ElementState::Pressed;
                let world = self.canvas.camera.screen_to_world(position);
                self.send_plugin_event(json!({"name":"pointer","phase":phase,"x":world.x,"y":world.y,"shift":shift,"ctrl":ctrl}))
            }
            WindowEvent::CursorMoved { .. } if self.plugin_pointer_captured => {
                let world = self.canvas.camera.screen_to_world(position);
                self.send_plugin_event(json!({"name":"pointer","phase":"move","x":world.x,"y":world.y,"shift":shift,"ctrl":ctrl}))
            }
            WindowEvent::KeyboardInput { event, .. }
                if self.plugins.selected.is_some() && event.state == ElementState::Pressed =>
            {
                if self.plugin_ime_composing {
                    return true;
                }
                let key = match &event.logical_key {
                    Key::Named(k) => format!("{k:?}"),
                    Key::Character(c) => c.to_lowercase(),
                    _ => String::new(),
                };
                let handled = self.send_plugin_event(
                    json!({"name":"key","key":key,"ctrl":ctrl,"shift":shift,"alt":mods.alt_key()}),
                );
                if !ctrl && !mods.alt_key() {
                    if let Some(text) = &event.text {
                        if !text.is_empty() && text.chars().all(|c| !c.is_control()) {
                            self.send_plugin_event(json!({"name":"text","text":text.as_str()}));
                        }
                    }
                }
                handled
            }
            _ => false,
        }
    }
    // TODO
    pub(crate) fn poll_plugin_requests(&mut self) -> bool {
        let requests = plugins::platform::drain();
        let changed = !requests.is_empty();
        for req in requests {
            match req {
                plugins::platform::Request::Document(bytes) => {
                    self.clear_plugin_tool();
                    match self.canvas.load_document_bytes(&bytes) {
                        Ok(()) => {
                            self.plugins.document_changed(&mut self.canvas);
                            self.plugins.notice = "Document opened".into();
                        }
                        Err(e) => self.plugins.notice = format!("Could not open document: {e:#}"),
                    }
                }
                plugins::platform::Request::Package(bytes) => {
                    match plugins::platform::install_zip(&bytes) {
                        Ok(id) => {
                            self.clear_plugin_tool();
                            self.plugins.rescan();
                            self.plugin_page.open = true;
                            self.plugin_page.selected = Some(id);
                            self.plugins.notice =
                                "Installed, not activated. Review permissions and activate.".into();
                        }
                        Err(e) => self.plugins.notice = format!("Import failed: {e:#}"),
                    }
                }
                plugins::platform::Request::Error(e) => self.plugins.notice = e,
                plugins::platform::Request::Event(e) => {
                    if !self.plugin_page.open && !self.is_capturing() {
                        let global = e["name"] == "key" && e["ctrl"] == true;
                        if global && e["key"] == "z" {
                            if e["shift"] == true {
                                self.redo();
                            } else {
                                self.undo();
                            }
                        } else if global && e["key"] == "y" {
                            self.redo();
                        } else if global && e["key"] == "s" {
                            self.plugin_action(UiAction::Save);
                        } else if global && e["key"] == "o" {
                            self.plugin_action(UiAction::Open);
                        } else if global && e["key"] == "p" && e["shift"] == true {
                            self.plugin_action(UiAction::TogglePage);
                        } else {
                            self.send_plugin_event(e);
                        }
                    }
                }
            }
        }
        self.sync_plugin_focus();
        changed
    }
}
