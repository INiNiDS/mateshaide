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

mod app;
pub use app::{initialize_input, App};

#[cfg(target_arch = "wasm32")]
mod wasm_entry;