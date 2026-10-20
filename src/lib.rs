pub mod camera;
pub mod canvas;
pub mod color;
pub mod engine;
pub mod input;
pub mod palette;
pub mod render_pipeline;
pub mod renderer;
pub mod shape;
pub mod test_spawn;
pub mod utils;
pub mod draw;
pub mod text;
pub mod builtin_input;

mod app;
pub use app::{App};
pub use builtin_input::initialize_input;
#[cfg(target_arch = "wasm32")]
mod wasm_entry;