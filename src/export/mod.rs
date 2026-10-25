use tracing::{error, info};
use crate::Engine;

pub mod bounds;
pub mod pdf;
pub mod png;
pub mod save;

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

pub fn export_to_file(engine: &mut Engine, format: ExportFormat) {
    let (result, ext, mime) = match format {
        ExportFormat::Png => (engine.export_png(), "png", "image/png"),
        ExportFormat::Pdf => (engine.export_pdf(), "pdf", "application/pdf"),
    };

    match result {
        Ok(bytes) => {
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let filename = format!("mateshaide-{stamp}.{ext}");
            save::save_bytes(&bytes, mime, &filename);
        }
        Err(e) => error!("export {ext} failed: {e}"),
    }
}

pub fn save_capture(png: &[u8]) {
    save::save_with_dialog(png, "image/png", "mateshaide-capture.png");
}

pub fn copy_capture(rgba: &[u8], width: u32, height: u32) {
    save::copy_image(rgba, width, height);
}
