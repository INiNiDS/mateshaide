use crate::{
    Engine,
    plugins::{self, Scene, UiAction, UiFrame},
};
use glam::Vec2;
use serde_json::{Value, json};
use winit::event::KeyEvent;
use winit::{
    event::{ElementState, Ime, MouseButton, MouseScrollDelta, WindowEvent},
    keyboard::{Key, KeyCode, ModifiersState, PhysicalKey},
};

const PLUGIN_WHEEL_LINE: f32 = 40.0;

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

        if overlay {
            let theme = self.plugins.settings.appearance.theme();

            for shape in &mut scene.shapes {
                shape.color = theme.display_ink(shape.color);
            }

            for text in &mut scene.texts {
                text.color = theme.display_ink(text.color);
            }
        }

        scene
    }

    pub(crate) fn plugin_frame(&self) -> UiFrame {
        if self.is_capturing() {
            UiFrame::with_theme(self.plugins.settings.appearance.theme())
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

            UiAction::SelectPlugin(id) => {
                self.plugin_page.selected = Some(id);
            }

            UiAction::TogglePlugin(id) => {
                self.plugin_page.dialog_scroll = 0.0;

                if self.plugins.active(&id) {
                    self.plugins.request_disable(&id);
                } else {
                    self.plugins.request_enable(&id);
                }
            }

            UiAction::Confirm => {
                self.plugins.confirm();
            }

            UiAction::Cancel => {
                self.plugins.confirmation = None;
            }

            UiAction::Rescan => {
                self.clear_plugin_tool();
                self.plugins.rescan();
            }

            UiAction::Repository(url) => {
                plugins::platform::open_repository(&url);
            }

            UiAction::Tool(plugin_id, tool_id) => {
                self.draw.end(&mut self.canvas);
                self.draw.enabled = false;

                #[cfg(debug_assertions)]
                self.disable_test_tools();

                self.plugin_pointer_captured = false;

                self.plugins
                    .select_tool(&plugin_id, &tool_id, &mut self.canvas);
            }

            UiAction::Command(plugin_id, command_id) => {
                self.plugins.dispatch(
                    &plugin_id,
                    json!({
                        "name": "command",
                        "id": command_id
                    }),
                    &mut self.canvas,
                );
            }

            UiAction::Save => match self.canvas.document_bytes() {
                Ok(bytes) => {
                    plugins::platform::save_document(&bytes);
                }
                Err(error) => {
                    self.plugins.notice = error.to_string();
                }
            },

            UiAction::Open => {
                plugins::platform::request_open();
            }

            UiAction::Import => {
                plugins::platform::request_import();
            }

            UiAction::ToggleDark => {
                self.toggle_appearance(true);
            }

            UiAction::ToggleHalloween => {
                self.toggle_appearance(false);
            }
        }

        self.sync_plugin_focus();
    }

    fn send_plugin_event(&mut self, event: Value) -> bool {
        if let Some((plugin_id, _)) = self.plugins.selected.clone() {
            self.plugins.dispatch(&plugin_id, event, &mut self.canvas);

            self.sync_plugin_focus();

            true
        } else {
            false
        }
    }

    pub(crate) fn handle_plugin_event(
        &mut self,
        event: &WindowEvent,
        cursor: Vec2,
        mods: ModifiersState,
    ) -> bool {
        let ctrl = mods.control_key() || mods.super_key();
        let shift = mods.shift_key();

        let position = match event {
            WindowEvent::CursorMoved { position, .. } => {
                Vec2::new(position.x as f32, position.y as f32)
            }
            _ => cursor,
        };

        self.capture_cursor = position;

        if let Some(result) = self.handle_confirmation_keys(event) {
            return result;
        }

        if let Some(result) = self.handle_page_shortcuts(event, ctrl, shift) {
            return result;
        }

        if let Some(result) = self.handle_ctrl_blocking(event, ctrl) {
            return result;
        }

        if let Some(result) = self.handle_pointer_and_wheel(event, position) {
            return result;
        }

        self.forward_event_to_plugin(event, position, ctrl, shift, mods)
    }

    fn handle_confirmation_keys(&mut self, event: &WindowEvent) -> Option<bool> {
        if self.plugins.confirmation.is_none() {
            return None;
        }
        if let WindowEvent::KeyboardInput { event, .. } = event {
            if event.state == ElementState::Pressed
                && event.physical_key == PhysicalKey::Code(KeyCode::Escape)
            {
                self.plugins.confirmation = None;
            }
            return Some(true);
        }
        None
    }

    fn handle_page_shortcuts(
        &mut self,
        event: &WindowEvent,
        ctrl: bool,
        shift: bool,
    ) -> Option<bool> {
        let WindowEvent::KeyboardInput { event, .. } = event else {
            return None;
        };
        if event.state != ElementState::Pressed {
            return None;
        }
        let PhysicalKey::Code(code) = event.physical_key else {
            return None;
        };

        if code == KeyCode::KeyP && ctrl && shift {
            self.plugin_action(UiAction::TogglePage);
            return Some(true);
        }
        if code == KeyCode::KeyS && ctrl {
            self.plugin_action(UiAction::Save);
            return Some(true);
        }
        if code == KeyCode::KeyO && ctrl {
            self.plugin_action(UiAction::Open);
            return Some(true);
        }
        if code == KeyCode::F5 && self.plugin_page.open {
            self.plugin_action(UiAction::Rescan);
            return Some(true);
        }
        if code == KeyCode::Escape && self.plugin_page.open {
            if self.plugins.confirmation.is_some() {
                self.plugins.confirmation = None;
            } else {
                self.plugin_action(UiAction::TogglePage);
            }
            return Some(true);
        }
        None
    }

    fn handle_ctrl_blocking(&mut self, event: &WindowEvent, ctrl: bool) -> Option<bool> {
        if !ctrl {
            return None;
        }
        let WindowEvent::KeyboardInput { event, .. } = event else {
            return None;
        };
        if event.state != ElementState::Pressed {
            return None;
        }
        let PhysicalKey::Code(code) = event.physical_key else {
            return None;
        };

        if matches!(
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
        ) {
            return Some(self.plugin_page.open);
        }
        if code == KeyCode::KeyX && !self.plugins.wants_text_input() {
            return Some(self.plugin_page.open);
        }
        None
    }

    fn handle_pointer_and_wheel(&mut self, event: &WindowEvent, position: Vec2) -> Option<bool> {
        let consumed = self.handle_ui_left_click(event, position)
            || self.handle_quick_panel_scroll(event, position)
            || self.handle_pan_over_frame(event, position)
            || self.handle_open_page_scroll(event);

        if consumed { Some(true) } else { None }
    }

    fn handle_ui_left_click(&mut self, event: &WindowEvent, position: Vec2) -> bool {
        let WindowEvent::MouseInput {
            button: MouseButton::Left,
            state: ElementState::Pressed,
            ..
        } = event
        else {
            return false;
        };

        let frame = self.plugin_frame();

        if let Some(action) = frame.hit(position) {
            self.plugin_action(action);
            return true;
        }

        frame.blocks_pointer(position)
    }

    fn handle_quick_panel_scroll(&mut self, event: &WindowEvent, position: Vec2) -> bool {
        if self.plugin_page.open || self.plugin_pointer_captured {
            return false;
        }

        let WindowEvent::MouseWheel { delta, .. } = event else {
            return false;
        };

        if self
            .plugins
            .quick_contains(&self.plugin_page, &self.canvas.camera, position)
        {
            let delta = Self::wheel_delta_y(delta);
            let max_scroll = self.plugins.quick_scroll_max(&self.canvas.camera);

            self.plugin_page.quick_scroll =
                (self.plugin_page.quick_scroll - delta).clamp(0.0, max_scroll);

            return true;
        }

        self.plugin_frame().blocks_pointer(position)
    }

    fn handle_pan_over_frame(&mut self, event: &WindowEvent, position: Vec2) -> bool {
        if self.plugin_page.open || self.plugin_pointer_captured {
            return false;
        }

        if !matches!(
            event,
            WindowEvent::MouseInput {
                button: MouseButton::Right | MouseButton::Middle,
                state: ElementState::Pressed,
                ..
            }
        ) {
            return false;
        }

        self.plugin_frame().blocks_pointer(position)
    }

    fn handle_open_page_scroll(&mut self, event: &WindowEvent) -> bool {
        if !self.plugin_page.open {
            return false;
        }

        if let WindowEvent::MouseWheel { delta, .. } = event {
            let delta = Self::wheel_delta_y(delta);
            let max_scroll = self
                .plugins
                .ui_scroll_max(&self.plugin_page, &self.canvas.camera);

            let scroll = if self.plugins.confirmation.is_some() {
                &mut self.plugin_page.dialog_scroll
            } else {
                &mut self.plugin_page.scroll
            };

            *scroll = (*scroll - delta).clamp(0.0, max_scroll);
        }

        true
    }

    fn forward_event_to_plugin(
        &mut self,
        event: &WindowEvent,
        position: Vec2,
        ctrl: bool,
        shift: bool,
        mods: ModifiersState,
    ) -> bool {
        match event {
            WindowEvent::Focused(false) => self.handle_focus_lost(),

            WindowEvent::Ime(ime) => self.handle_ime(ime),

            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                ..
            } if self.plugins.selected.is_some()
                && (*state == ElementState::Pressed || self.plugin_pointer_captured) =>
            {
                let phase = if *state == ElementState::Pressed {
                    "down"
                } else {
                    "up"
                };
                self.plugin_pointer_captured = *state == ElementState::Pressed;
                self.send_pointer_event(phase, position, ctrl, shift)
            }

            WindowEvent::CursorMoved { .. } if self.plugin_pointer_captured => {
                self.send_pointer_event("move", position, ctrl, shift)
            }

            WindowEvent::KeyboardInput { event, .. }
                if self.plugins.selected.is_some() && event.state == ElementState::Pressed =>
            {
                self.handle_plugin_key(event, ctrl, shift, mods)
            }

            _ => false,
        }
    }

    fn wheel_delta_y(delta: &MouseScrollDelta) -> f32 {
        match delta {
            MouseScrollDelta::LineDelta(_, y) => *y * PLUGIN_WHEEL_LINE,
            MouseScrollDelta::PixelDelta(position) => position.y as f32,
        }
    }

    fn handle_focus_lost(&mut self) -> bool {
        self.plugins.blur(&mut self.canvas);
        self.plugin_pointer_captured = false;
        self.plugin_ime_composing = false;
        self.sync_plugin_focus();
        false
    }

    fn handle_ime(&mut self, ime: &Ime) -> bool {
        match ime {
            Ime::Enabled => false,

            Ime::Preedit(text, _) => {
                self.plugin_ime_composing = !text.is_empty();
                self.send_plugin_event(json!({
                    "name": "ime.preedit",
                    "text": text
                }))
            }

            Ime::Commit(text) => {
                self.plugin_ime_composing = false;
                self.send_plugin_event(json!({
                    "name": "text",
                    "text": text
                }))
            }

            Ime::Disabled => {
                self.plugin_ime_composing = false;
                self.send_plugin_event(json!({
                    "name": "ime.preedit",
                    "text": ""
                }))
            }
        }
    }

    fn send_pointer_event(&mut self, phase: &str, position: Vec2, ctrl: bool, shift: bool) -> bool {
        let world = self.canvas.camera.screen_to_world(position);

        self.send_plugin_event(json!({
            "name": "pointer",
            "phase": phase,
            "x": world.x,
            "y": world.y,
            "shift": shift,
            "ctrl": ctrl
        }))
    }

    fn handle_plugin_key(
        &mut self,
        event: &KeyEvent,
        ctrl: bool,
        shift: bool,
        mods: ModifiersState,
    ) -> bool {
        if self.plugin_ime_composing {
            return true;
        }

        let key = match &event.logical_key {
            Key::Named(key) => format!("{key:?}"),
            Key::Character(character) => character.to_lowercase(),
            _ => String::new(),
        };

        let handled = self.send_plugin_event(json!({
            "name": "key",
            "key": key,
            "ctrl": ctrl,
            "shift": shift,
            "alt": mods.alt_key()
        }));

        if !ctrl && !mods.alt_key() {
            if let Some(text) = &event.text {
                if !text.is_empty() && text.chars().all(|c| !c.is_control()) {
                    self.send_plugin_event(json!({
                        "name": "text",
                        "text": text.as_str()
                    }));
                }
            }
        }

        handled
    }

    pub(crate) fn poll_plugin_requests(&mut self) -> bool {
        let requests = plugins::platform::drain();
        let changed = !requests.is_empty();

        for request in requests {
            match request {
                plugins::platform::Request::Document(bytes) => self.open_plugin_document(bytes),
                plugins::platform::Request::Package(bytes) => self.install_plugin_package(bytes),
                plugins::platform::Request::Error(error) => self.plugins.notice = error,
                plugins::platform::Request::Event(event) => self.handle_plugin_message(event),
            }
        }

        self.sync_plugin_focus();

        changed
    }

    fn open_plugin_document(&mut self, bytes: Vec<u8>) {
        self.clear_plugin_tool();

        match self.canvas.load_document_bytes(&bytes) {
            Ok(()) => {
                self.plugins.document_changed(&mut self.canvas);
                self.plugins.notice = "Document opened".into();
            }
            Err(error) => {
                self.plugins.notice = format!("Could not open document: {error:#}");
            }
        }
    }

    fn install_plugin_package(&mut self, bytes: Vec<u8>) {
        match plugins::platform::install_zip(&bytes) {
            Ok(id) => {
                self.clear_plugin_tool();
                self.plugins.rescan();

                self.plugin_page.open = true;
                self.plugin_page.selected = Some(id);

                self.plugins.notice =
                    "Installed, not activated. Review permissions and activate.".into();
            }
            Err(error) => {
                self.plugins.notice = format!("Import failed: {error:#}");
            }
        }
    }

    fn handle_plugin_message(&mut self, event: Value) {
        if self.plugin_page.open || self.is_capturing() {
            return;
        }

        let global = event["name"] == "key" && event["ctrl"] == true;

        if !global || !self.apply_plugin_global_shortcut(&event) {
            self.send_plugin_event(event);
        }
    }

    fn apply_plugin_global_shortcut(&mut self, event: &Value) -> bool {
        let key = event["key"].as_str();
        let shift = event["shift"] == true;

        match (key, shift) {
            (Some("z"), false) => {
                self.undo();
                true
            }
            (Some("z"), true) | (Some("y"), _) => {
                self.redo();
                true
            }
            (Some("s"), _) => {
                self.plugin_action(UiAction::Save);
                true
            }
            (Some("o"), _) => {
                self.plugin_action(UiAction::Open);
                true
            }
            (Some("p"), true) => {
                self.plugin_action(UiAction::TogglePage);
                true
            }
            _ => false,
        }
    }

    fn toggle_appearance(&mut self, dark: bool) {
        self.draw.end(&mut self.canvas);

        let appearance = &mut self.plugins.settings.appearance;

        if dark {
            appearance.dark = !appearance.dark;
        } else {
            appearance.halloween = !appearance.halloween;
        }

        self.plugins.persist();

        plugins::platform::sync_appearance(self.plugins.settings.appearance);
    }
}
