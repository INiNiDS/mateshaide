use crate::export::ExportFormat;
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
const DOUBLE_CLICK_SECS: f32 = 0.4;
const DOUBLE_CLICK_PX: f32 = 6.0;

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

            if show_overlay {
                scene.shapes.extend(self.eraser_highlight());
            }
        }

        scene
    }

    pub(crate) fn plugin_frame(&mut self) -> UiFrame {
        self.plugin_page.tools.draw = self.draw.enabled;
        self.plugin_page.tools.eraser = self.eraser.enabled;
        self.plugin_page.tools.text = self.text_tool_selected();
        if !self.plugin_page.open && !self.is_capturing() {
            self.plugins.refresh_panels(&self.canvas.camera);
        }
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

            UiAction::ToggleText => {
                if self.text_tool_selected() {
                    self.clear_plugin_tool();
                } else {
                    self.select_text_tool();
                }
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
                self.plugin_page.tools.draw = false;

                #[cfg(debug_assertions)]
                self.disable_test_tools();

                self.set_eraser(false);

                self.plugin_pointer_captured = false;

                self.plugins
                    .select_tool(&plugin_id, &tool_id, &mut self.canvas);
            }

            UiAction::Command(plugin_id, command_id) => {
                self.plugins.dispatch(
                    &plugin_id,
                    &json!({
                        "name": "command",
                        "id": command_id
                    }),
                    &mut self.canvas,
                );
            }

            UiAction::PanelButton(plugin_id, button_id) => self.plugins.dispatch(
                &plugin_id,
                &json!({"name": "panel.click", "id": button_id}),
                &mut self.canvas,
            ),

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

            UiAction::ToggleDraw => {
                self.clear_plugin_tool();
                self.draw.end(&mut self.canvas);
                #[cfg(debug_assertions)]
                if !self.draw.enabled {
                    self.disable_test_tools();
                }
                if !self.draw.enabled {
                    self.set_eraser(false);
                }
                self.draw.toggle();
                self.plugin_page.tools.draw = self.draw.enabled;
            }

            UiAction::ToggleEraser => {
                self.toggle_eraser();
            }

            UiAction::Undo => {
                self.undo();
            }

            UiAction::Redo => {
                self.redo();
            }

            UiAction::ZoomIn => {
                #[cfg(debug_assertions)]
                self.finish_test_interaction();
                self.canvas.zoom_at_center(1.0);
            }

            UiAction::ZoomOut => {
                #[cfg(debug_assertions)]
                self.finish_test_interaction();
                self.canvas.zoom_at_center(-1.0);
            }

            UiAction::ResetView => {
                #[cfg(debug_assertions)]
                self.finish_test_interaction();
                self.canvas.reset_view();
            }

            UiAction::ExportPng => {
                self.begin_png_capture();
            }

            UiAction::ExportPdf => {
                crate::export::export_to_file(self, ExportFormat::Pdf);
            }
        }

        self.sync_plugin_focus();
    }

    fn send_plugin_event(&mut self, event: &Value) -> bool {
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
        self.plugins.confirmation.as_ref()?;
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

        if code == KeyCode::F1
            && !self.plugin_page.open
            && self.plugins.active(plugins::HELP_PLUGIN)
        {
            self.plugin_action(UiAction::Command(
                plugins::HELP_PLUGIN.into(),
                "toggle".into(),
            ));
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
            KeyCode::KeyZ | KeyCode::KeyY | KeyCode::KeyE | KeyCode::KeyP
        ) {
            return Some(self.plugin_page.open);
        }
        if !self.plugins.wants_text_input()
            && matches!(
                code,
                KeyCode::KeyX
                    | KeyCode::Digit9
                    | KeyCode::Equal
                    | KeyCode::Minus
                    | KeyCode::Digit0
                    | KeyCode::NumpadAdd
                    | KeyCode::NumpadSubtract
            )
        {
            return Some(self.plugin_page.open);
        }
        None
    }

    fn handle_pointer_and_wheel(&mut self, event: &WindowEvent, position: Vec2) -> Option<bool> {
        let consumed = self.handle_panel_wheel(event, position)
            || self.handle_ui_left_click(event, position)
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

            WindowEvent::MouseWheel { delta, .. } if self.plugin_wants_wheel(position) => {
                let world = self.canvas.camera.screen_to_world(position);
                self.send_plugin_event(&json!({
                    "name": "wheel",
                    "x": world.x,
                    "y": world.y,
                    "delta": Self::wheel_delta_y(delta),
                    "ctrl": ctrl,
                    "shift": shift
                }))
            }

            WindowEvent::KeyboardInput { event, .. }
                if self.plugins.selected.is_some()
                    && event.state == ElementState::Pressed
                    && !self.is_tool_shortcut(event, ctrl, mods) =>
            {
                self.handle_plugin_key(event, ctrl, shift, mods)
            }

            _ => false,
        }
    }

    fn plugin_wants_wheel(&self, position: Vec2) -> bool {
        let Some((plugin, tool)) = &self.plugins.selected else {
            return false;
        };
        let wants = self.plugins.entries.get(plugin).is_some_and(|e| {
            e.contributions
                .tools
                .iter()
                .any(|t| t.id == *tool && t.wheel)
        });
        let p = self.canvas.camera.screen_to_world(position);
        wants
            && self.canvas.plugin_nodes.iter().any(|n| {
                n.owner == *plugin
                    && p.x >= n.x
                    && p.x <= n.x + n.width
                    && p.y <= n.y
                    && p.y >= n.y - n.height
            })
    }

    #[must_use]
    pub fn text_tool_selected(&self) -> bool {
        self.plugins
            .selected
            .as_ref()
            .is_some_and(|(p, t)| p == plugins::TEXT_PLUGIN && t == plugins::TEXT_TOOL)
    }

    pub fn select_text_tool(&mut self) -> bool {
        if self.text_tool_selected() {
            return true;
        }
        if !self.plugins.active(plugins::TEXT_PLUGIN) {
            self.plugins.notice =
                "Text tool needs the Text Editor plugin: open Plugins and activate it.".into();
            return false;
        }
        self.plugin_action(UiAction::Tool(
            plugins::TEXT_PLUGIN.into(),
            plugins::TEXT_TOOL.into(),
        ));
        self.text_tool_selected()
    }

    pub fn create_text_at(&mut self, screen: Vec2) -> bool {
        if !self.select_text_tool() {
            return false;
        }
        self.send_pointer_event("down", screen, false, false);
        self.send_pointer_event("up", screen, false, false);
        true
    }

    pub fn register_canvas_click(&mut self, screen: Vec2) -> bool {
        let now = web_time::Instant::now();
        let double = self.last_click.is_some_and(|(at, pos)| {
            (now - at).as_secs_f32() <= DOUBLE_CLICK_SECS && pos.distance(screen) <= DOUBLE_CLICK_PX
        });
        self.last_click = if double { None } else { Some((now, screen)) };
        double
    }

    fn is_tool_shortcut(&self, event: &KeyEvent, ctrl: bool, mods: ModifiersState) -> bool {
        !ctrl
            && !mods.alt_key()
            && !self.plugins.wants_text_input()
            && matches!(
                event.physical_key,
                PhysicalKey::Code(KeyCode::KeyE | KeyCode::Escape | KeyCode::KeyT)
            )
    }

    fn wheel_delta_y(delta: &MouseScrollDelta) -> f32 {
        match delta {
            MouseScrollDelta::LineDelta(_, y) => *y * PLUGIN_WHEEL_LINE,
            MouseScrollDelta::PixelDelta(position) => position.y as f32,
        }
    }

    fn handle_focus_lost(&mut self) -> bool {
        tracing::info!(
            "plugin focus lost, wants_text_input={}",
            self.plugins.wants_text_input()
        );
        // On web the hidden textarea takes focus from the canvas; that is not a real blur.
        if plugins::platform::browser_editor_has_focus() {
            return false;
        }

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
                self.send_plugin_event(&json!({
                    "name": "ime.preedit",
                    "text": text
                }))
            }

            Ime::Commit(text) => {
                self.plugin_ime_composing = false;
                self.send_plugin_event(&json!({
                    "name": "text",
                    "text": text
                }))
            }

            Ime::Disabled => {
                self.plugin_ime_composing = false;
                self.send_plugin_event(&json!({
                    "name": "ime.preedit",
                    "text": ""
                }))
            }
        }
    }

    fn send_pointer_event(&mut self, phase: &str, position: Vec2, ctrl: bool, shift: bool) -> bool {
        let world = self.canvas.camera.screen_to_world(position);

        self.send_plugin_event(&json!({
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

        let key = match event.physical_key {
            PhysicalKey::Code(KeyCode::Digit0) => "0".to_owned(),
            PhysicalKey::Code(KeyCode::Digit1) => "1".to_owned(),
            PhysicalKey::Code(KeyCode::Digit2) => "2".to_owned(),
            PhysicalKey::Code(KeyCode::Digit3) => "3".to_owned(),
            PhysicalKey::Code(KeyCode::Digit4) => "4".to_owned(),
            PhysicalKey::Code(KeyCode::Digit5) => "5".to_owned(),
            PhysicalKey::Code(KeyCode::Digit6) => "6".to_owned(),
            PhysicalKey::Code(KeyCode::Digit7) => "7".to_owned(),
            PhysicalKey::Code(KeyCode::Digit8) => "8".to_owned(),
            PhysicalKey::Code(KeyCode::Digit9) => "9".to_owned(),
            _ => match &event.logical_key {
                Key::Named(key) => format!("{key:?}"),
                Key::Character(character) => character.to_lowercase(),
                _ => String::new(),
            },
        };

        let handled = self.send_plugin_event(&json!({
            "name": "key",
            "key": key,
            "ctrl": ctrl,
            "shift": shift,
            "alt": mods.alt_key()
        }));

        if !ctrl
            && !mods.alt_key()
            && let Some(text) = &event.text
            && !text.is_empty()
            && text.chars().all(|c| !c.is_control())
        {
            self.send_plugin_event(&json!({
                "name": "text",
                "text": text.as_str(),
                "shift": shift
            }));
        }

        handled
    }

    pub(crate) fn poll_plugin_requests(&mut self) -> bool {
        let requests = plugins::platform::drain();
        let changed = !requests.is_empty();

        for request in requests {
            match request {
                plugins::platform::Request::Document(bytes) => self.open_plugin_document(&bytes),
                plugins::platform::Request::Package(bytes) => self.install_plugin_package(&bytes),
                plugins::platform::Request::PluginFile(file) => {
                    self.plugins.deliver_file(file, &mut self.canvas);
                }
                plugins::platform::Request::Error(error) => self.plugins.notice = error,
                plugins::platform::Request::Event(event) => self.handle_plugin_message(&event),
            }
        }

        self.sync_plugin_focus();

        changed
    }

    fn open_plugin_document(&mut self, bytes: &[u8]) {
        self.clear_plugin_tool();

        match self.canvas.load_document_bytes(bytes) {
            Ok(()) => {
                self.plugins.document_changed(&mut self.canvas);
                self.plugins.notice = "Document opened".into();
            }
            Err(error) => {
                self.plugins.notice = format!("Could not open document: {error:#}");
            }
        }
    }

    fn install_plugin_package(&mut self, bytes: &[u8]) {
        match plugins::platform::install_zip(bytes) {
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

    fn handle_plugin_message(&mut self, event: &Value) {
        if self.plugin_page.open || self.is_capturing() {
            return;
        }

        let global = event["name"] == "key" && event["ctrl"] == true;

        if !global || !self.apply_plugin_global_shortcut(event) {
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
