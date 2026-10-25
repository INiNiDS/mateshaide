use crate::export::ExportError;
use crate::export::bounds::Rect;
use image::{DynamicImage, ImageFormat, RgbaImage};
use std::io::Cursor;
use web_time::Instant;

const SUPERSAMPLE: f32 = 2.0;
const MIN_SIDE: f32 = 256.0;
const MAX_SIDE: f32 = 4096.0;

pub struct CapturedPng {
    pub png: Vec<u8>,
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

pub enum CaptureState {
    Idle,
    Converge {
        started: Instant,
        image: CapturedPng,
    },
    Preview {
        image: CapturedPng,
        shown_at: Instant,
    },
}

impl Default for CaptureState {
    fn default() -> Self {
        CaptureState::Idle
    }
}

impl CaptureState {
    pub fn is_idle(&self) -> bool {
        matches!(self, CaptureState::Idle)
    }

    pub fn is_preview(&self) -> bool {
        matches!(self, CaptureState::Preview { .. })
    }
}

#[must_use]
pub fn pixel_size(frame: Rect) -> (u32, u32) {
    let mut target = frame.size() * SUPERSAMPLE;
    let longest = target.max_element();
    if longest > MAX_SIDE {
        target *= MAX_SIDE / longest;
    } else if longest < MIN_SIDE {
        target *= MIN_SIDE / longest;
    }
    (
        target.x.round().max(1.0) as u32,
        target.y.round().max(1.0) as u32,
    )
}

pub fn encode(rgba: &[u8], width: u32, height: u32) -> Result<Vec<u8>, ExportError> {
    let image = RgbaImage::from_raw(width, height, rgba.to_vec()).ok_or(ExportError::Buffer)?;
    let mut out = Vec::new();
    DynamicImage::ImageRgba8(image).write_to(&mut Cursor::new(&mut out), ImageFormat::Png)?;
    Ok(out)
}
