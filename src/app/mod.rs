pub mod actions;
#[cfg(debug_assertions)]
pub mod demo;
pub mod events;
pub mod lifecycle;

#[cfg(debug_assertions)]
use crate::dev::actions as dev_actions;
use crate::engine::Engine;
use crate::input::InputManager;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use winit::window::Window;

type Slot<T> = Rc<RefCell<Option<T>>>;

pub struct App {
    window: Slot<Arc<Window>>,
    engine: Slot<Engine>,
    input: InputManager,
}

impl App {
    pub fn new() -> Self {
        let mut input = InputManager::new();

        actions::register(&mut input);
        #[cfg(debug_assertions)]
        dev_actions::register(&mut input);

        Self {
            window: Rc::new(RefCell::new(None)),
            engine: Rc::new(RefCell::new(None)),
            input,
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
