pub mod overlay;
pub mod pipeline;
pub mod renderer;
pub mod text_renderer;

use overlay::{build_swatch_instances, OVERLAY_CAP};

pub use pipeline::RenderPipeline;
pub use renderer::{Renderer, RendererError};
pub use text_renderer::TextLayer;