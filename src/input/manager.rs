use crate::core::collections::{FastHashMap, FastHashSet};
use crate::engine::Engine;
use crate::input::action::{ActionCallback, ActionID};
use crate::input::shortcut::{InputTrigger, Shortcut};
use crate::input::update::InputUpdate;
use glam::Vec2;
use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent};
use winit::keyboard::{KeyCode, ModifiersState, PhysicalKey};

const TOUCHPAD_SCROLL_SCALE: f32 = 0.05;

pub struct InputManager<Ctx = Engine> {
    actions: FastHashMap<ActionID, ActionCallback<Ctx>>,
    bindings: FastHashMap<Shortcut, ActionID>,

    key_down: FastHashSet<KeyCode>,
    mouse_buttons_down: FastHashSet<MouseButton>,
    cursor_pos: Vec2,
    modifiers: ModifiersState,
}

impl<Ctx> Default for InputManager<Ctx> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Ctx> InputManager<Ctx> {
    #[must_use]
    pub fn new() -> Self {
        Self {
            actions: FastHashMap::default(),
            bindings: FastHashMap::default(),
            key_down: FastHashSet::default(),
            mouse_buttons_down: FastHashSet::default(),
            cursor_pos: Vec2::ZERO,
            modifiers: ModifiersState::default(),
        }
    }

    pub fn register_action<I, F>(&mut self, id: I, callback: F)
    where
        I: Into<ActionID>,
        F: FnMut(&mut Ctx) + 'static,
    {
        self.actions.insert(id.into(), Box::new(callback));
    }

    pub fn bind<I: Into<ActionID>>(&mut self, shortcut: Shortcut, action_id: I) {
        self.bindings.insert(shortcut, action_id.into());
    }

    pub fn unbind(&mut self, shortcut: Shortcut) {
        self.bindings.remove(&shortcut);
    }

    pub fn remove_action<I: Into<ActionID>>(&mut self, action_id: I) {
        let action_id = action_id.into();
        self.actions.remove(&action_id);
        self.bindings.retain(|_, v| v != &action_id);
    }

    /// Dispatches an action right now by its ID, invoking the associated callback if it exists, or returns false.
    pub fn dispatch<I: Into<ActionID>>(&mut self, action_id: I, ctx: &mut Ctx) -> bool {
        let action_id = action_id.into();
        if let Some(callback) = self.actions.get_mut(&action_id) {
            callback(ctx);
            true
        } else {
            false
        }
    }

    #[must_use]
    pub fn is_key_down(&self, key: KeyCode) -> bool {
        self.key_down.contains(&key)
    }

    #[must_use]
    pub fn is_mouse_down(&self, button: MouseButton) -> bool {
        self.mouse_buttons_down.contains(&button)
    }

    #[must_use]
    pub fn cursor_position(&self) -> Vec2 {
        self.cursor_pos
    }

    #[must_use]
    pub fn modifiers(&self) -> ModifiersState {
        self.modifiers
    }

    #[must_use]
    pub fn is_ctrl(&self) -> bool {
        self.modifiers.control_key() || self.modifiers.super_key()
    }

    #[must_use]
    pub fn is_alt(&self) -> bool {
        self.modifiers.alt_key()
    }

    #[must_use]
    pub fn is_shift(&self) -> bool {
        self.modifiers.shift_key()
    }

    #[must_use]
    pub fn find_matching_action(&self, trigger: InputTrigger) -> Option<ActionID> {
        let shortcut = Shortcut {
            trigger,
            ctrl: self.is_ctrl(),
            shift: self.is_shift(),
            alt: self.is_alt(),
        };
        self.bindings.get(&shortcut).cloned()
    }

    pub fn process_event(&mut self, event: &WindowEvent, ctx: &mut Ctx) -> InputUpdate {
        match event {
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers.state();
                InputUpdate::None
            }
            WindowEvent::Focused(false) => {
                self.key_down.clear();
                self.mouse_buttons_down.clear();
                self.modifiers = ModifiersState::default();
                InputUpdate::FocusLost
            }
            WindowEvent::KeyboardInput { event, .. } => self.handle_keyboard(event, ctx),
            WindowEvent::CursorMoved { position, .. } => self.handle_cursor_moved(*position),
            WindowEvent::MouseInput { state, button, .. } => {
                self.handle_mouse_input(*button, *state, ctx)
            }
            WindowEvent::MouseWheel { delta, .. } => Self::handle_mouse_wheel(delta),
            _ => InputUpdate::None,
        }
    }

    fn handle_keyboard(&mut self, event: &KeyEvent, ctx: &mut Ctx) -> InputUpdate {
        let PhysicalKey::Code(key) = event.physical_key else {
            return InputUpdate::None;
        };
        match event.state {
            ElementState::Pressed => {
                self.key_down.insert(key);
                if self.try_trigger(InputTrigger::Key(key), ctx) {
                    return InputUpdate::ActionExecuted;
                }
            }
            ElementState::Released => {
                self.key_down.remove(&key);
            }
        }

        InputUpdate::None
    }

    fn handle_cursor_moved(&mut self, position: PhysicalPosition<f64>) -> InputUpdate {
        let new_pos = Vec2::new(position.x as f32, position.y as f32);
        let delta = new_pos - self.cursor_pos;
        self.cursor_pos = new_pos;

        InputUpdate::CursorMoved {
            position: new_pos,
            delta,
        }
    }

    fn handle_mouse_input(
        &mut self,
        button: MouseButton,
        state: ElementState,
        ctx: &mut Ctx,
    ) -> InputUpdate {
        match state {
            ElementState::Pressed => {
                self.mouse_buttons_down.insert(button);
                if self.try_trigger(InputTrigger::MouseButton(button), ctx) {
                    return InputUpdate::ActionExecuted;
                }
            }
            ElementState::Released => {
                self.mouse_buttons_down.remove(&button);
            }
        }

        InputUpdate::MouseButton { button, state }
    }

    fn handle_mouse_wheel(delta: &MouseScrollDelta) -> InputUpdate {
        let delta_vec = match *delta {
            MouseScrollDelta::LineDelta(x, y) => Vec2::new(x, y),
            MouseScrollDelta::PixelDelta(pos) => Vec2::new(
                pos.x as f32 * TOUCHPAD_SCROLL_SCALE,
                pos.y as f32 * TOUCHPAD_SCROLL_SCALE,
            ),
        };
        InputUpdate::MouseWheel { delta: delta_vec }
    }

    fn try_trigger(&mut self, trigger: InputTrigger, ctx: &mut Ctx) -> bool {
        let Some(action_id) = self.find_matching_action(trigger) else {
            return false;
        };
        if let Some(callback) = self.actions.get_mut(&action_id) {
            callback(ctx);
            true
        } else {
            false
        }
    }
}
