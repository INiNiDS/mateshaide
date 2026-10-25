use tracing::{error, info};
use crate::Engine;

pub mod bounds;
pub mod pdf;
pub mod png;

pub const PADDING: f32 = 32.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Png,
    Pdf,
}

#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    #[error("nothing to export: canvas is empty")]
    Empty,
    #[error("too many pages: {0} (max {max})", max = pdf::MAX_PAGES)]
    TooManyPages(usize),
    #[error("invalid image buffer")]
    Buffer,
    #[error("pdf generation failed: {0}")]
    Pdf(String),
    #[error(transparent)]
    Encode(#[from] image::ImageError),
    #[error(transparent)]
    Renderer(#[from] crate::graphics::RendererError),
}

#[cfg(not(target_arch = "wasm32"))]
pub fn export_to_file(engine: &mut Engine, format: ExportFormat) {
    let (result, ext) = match format {
        ExportFormat::Png => (pollster::block_on(engine.export_png()), "png"),
        ExportFormat::Pdf => (pollster::block_on(engine.export_pdf()), "pdf"),
    };

    match result {
        Ok(bytes) => {
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let path = format!("mateshaide-{stamp}.{ext}");
            match std::fs::write(&path, bytes) {
                Ok(()) => info!("exported to {path}"),
                Err(e) => error!("failed to write {path}: {e}"),
            }
        }
        Err(e) => error!("export {ext} failed: {e}"),
    }
}