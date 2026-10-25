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

#[cfg(not(target_arch = "wasm32"))]
pub fn save_capture_with_dialog(png: &[u8]) -> bool {
    let path = rfd::FileDialog::new()
        .add_filter("PNG Image", &["png"])
        .set_file_name("mateshaide-capture.png")
        .save_file();

    if let Some(path) = path {
        match std::fs::write(&path, png) {
            Ok(()) => {
                info!("saved capture to {}", path.display());
                true
            }
            Err(e) => {
                error!("failed to save capture: {e}");
                false
            }
        }
    } else {
        false
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn copy_capture_to_clipboard(rgba: &[u8], width: u32, height: u32) -> bool {
    match arboard::Clipboard::new() {
        Ok(mut clipboard) => {
            let img = arboard::ImageData {
                width: width as usize,
                height: height as usize,
                bytes: std::borrow::Cow::Borrowed(rgba),
            };
            match clipboard.set_image(img) {
                Ok(()) => {
                    info!("copied capture to clipboard");
                    true
                }
                Err(e) => {
                    error!("failed to copy to clipboard: {e}");
                    false
                }
            }
        }
        Err(e) => {
            error!("failed to open clipboard: {e}");
            false
        }
    }
}
