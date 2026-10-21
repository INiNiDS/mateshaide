pub mod action;
pub mod manager;
pub mod shortcut;
pub mod update;

pub use action::{ActionCallback, ActionID};
pub use manager::InputManager;
pub use shortcut::{InputTrigger, Shortcut};
pub use update::InputUpdate;