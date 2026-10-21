mod app;
pub mod canvas;
pub mod core;
pub(crate) mod dev;
pub mod engine;
pub mod graphics;
pub mod input;
pub mod shape;
pub mod theme;

pub use app::App;
pub use canvas::{Camera, CameraUniform, Canvas, CanvasText, DrawTool};
pub use core::Color;
pub use engine::{Engine, EngineError};
pub use graphics::{Renderer, RendererError};
pub use input::{ActionCallback, ActionID, InputManager, InputTrigger, InputUpdate, Shortcut};
pub use shape::{Sdf, ShapeError, ShapeInstance, ShapeRegistry};
pub use theme::palette;

#[cfg(target_arch = "wasm32")]
pub mod wasm;
