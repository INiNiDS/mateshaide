use winit::event::MouseButton;
use winit::keyboard::KeyCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InputTrigger {
    Key(KeyCode),
    MouseButton(MouseButton),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Shortcut {
    pub trigger: InputTrigger,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

impl Shortcut {
    #[must_use]
    pub fn key(key: KeyCode) -> Self {
        Self {
            trigger: InputTrigger::Key(key),
            ctrl: false,
            shift: false,
            alt: false,
        }
    }

    #[must_use]
    pub fn mouse(button: MouseButton) -> Self {
        Self {
            trigger: InputTrigger::MouseButton(button),
            ctrl: false,
            shift: false,
            alt: false,
        }
    }

    #[must_use]
    pub fn ctrl(mut self) -> Self {
        self.ctrl = true;
        self
    }

    #[must_use]
    pub fn shift(mut self) -> Self {
        self.shift = true;
        self
    }

    #[must_use]
    pub fn alt(mut self) -> Self {
        self.alt = true;
        self
    }
}
