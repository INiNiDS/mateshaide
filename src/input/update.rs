use glam::Vec2;
use winit::event::{ElementState, MouseButton};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputUpdate {
    None,
    ActionExecuted,
    FocusLost,
    CursorMoved {
        position: Vec2,
        delta: Vec2,
    },
    MouseWheel {
        delta: Vec2,
    },
    MouseButton {
        button: MouseButton,
        state: ElementState,
    },
}
