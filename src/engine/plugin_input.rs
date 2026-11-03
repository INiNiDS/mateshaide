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


    // TODO
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

        if self.plugins.confirmation.is_some() {
            if let WindowEvent::KeyboardInput { event, .. } = event {
                if event.state == ElementState::Pressed
                    && event.physical_key == PhysicalKey::Code(KeyCode::Escape)
                {
                    self.plugins.confirmation = None;
                }

                return true;
            }
        }

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
            let frame = self.plugin_frame();

            if let Some(action) = frame.hit(position) {
                self.plugin_action(action);
                return true;
            }

            if frame.blocks_pointer(position) {
                return true;
            }
        }

        if !self.plugin_page.open && !self.plugin_pointer_captured {
            if let WindowEvent::MouseWheel { delta, .. } = event {
                if self
                    .plugins
                    .quick_contains(&self.plugin_page, &self.canvas.camera, position)
                {
                    let delta = match delta {
                        MouseScrollDelta::LineDelta(_, y) => *y * 40.0,
                        MouseScrollDelta::PixelDelta(position) => position.y as f32,
                    };

                    let max_scroll = self.plugins.quick_scroll_max(&self.canvas.camera);

                    self.plugin_page.quick_scroll =
                        (self.plugin_page.quick_scroll - delta).clamp(0.0, max_scroll);

                    return true;
                }

                if self.plugin_frame().blocks_pointer(position) {
                    return true;
                }
            }

            if matches!(
                event,
                WindowEvent::MouseInput {
                    button: MouseButton::Right | MouseButton::Middle,
                    state: ElementState::Pressed,
                    ..
                }
            ) && self.plugin_frame().blocks_pointer(position)
            {
                return true;
            }
        }

        if self.plugin_page.open {
            if let WindowEvent::MouseWheel { delta, .. } = event {
                let delta = match delta {
                    MouseScrollDelta::LineDelta(_, y) => *y * 40.0,
                    MouseScrollDelta::PixelDelta(position) => position.y as f32,
                };

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

                self.send_plugin_event(json!({
                    "name": "ime.preedit",
                    "text": text
                }))
            }

            WindowEvent::Ime(Ime::Commit(text)) => {
                self.plugin_ime_composing = false;

                self.send_plugin_event(json!({
                    "name": "text",
                    "text": text
                }))
            }

            WindowEvent::Ime(Ime::Disabled) => {
                self.plugin_ime_composing = false;

                self.send_plugin_event(json!({
                    "name": "ime.preedit",
                    "text": ""
                }))
            }

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

            WindowEvent::CursorMoved { .. } if self.plugin_pointer_captured => {
                let world = self.canvas.camera.screen_to_world(position);

                self.send_plugin_event(json!({
                    "name": "pointer",
                    "phase": "move",
                    "x": world.x,
                    "y": world.y,
                    "shift": shift,
                    "ctrl": ctrl
                }))
            }

            WindowEvent::KeyboardInput { event, .. }
                if self.plugins.selected.is_some() && event.state == ElementState::Pressed =>
            {
                if self.plugin_ime_composing {
                    return true;
                }

                let key = match &event.logical_key {
                    Key::Named(key) => {
                        format!("{key:?}")
                    }
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

            _ => false,
        }
    }

    pub(crate) fn poll_plugin_requests(&mut self) -> bool {
        let requests = plugins::platform::drain();
        let changed = !requests.is_empty();

        for request in requests {
            match request {
                plugins::platform::Request::Document(bytes) => {
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

                        Err(error) => {
                            self.plugins.notice = format!("Import failed: {error:#}");
                        }
                    }
                }

                plugins::platform::Request::Error(error) => {
                    self.plugins.notice = error;
                }

                plugins::platform::Request::Event(event) => {
                    if !self.plugin_page.open && !self.is_capturing() {
                        let global = event["name"] == "key" && event["ctrl"] == true;

                        if global && event["key"] == "z" {
                            if event["shift"] == true {
                                self.redo();
                            } else {
                                self.undo();
                            }
                        } else if global && event["key"] == "y" {
                            self.redo();
                        } else if global && event["key"] == "s" {
                            self.plugin_action(UiAction::Save);
                        } else if global && event["key"] == "o" {
                            self.plugin_action(UiAction::Open);
                        } else if global && event["key"] == "p" && event["shift"] == true {
                            self.plugin_action(UiAction::TogglePage);
                        } else {
                            self.send_plugin_event(event);
                        }
                    }
                }
            }
        }

        self.sync_plugin_focus();

        changed
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
