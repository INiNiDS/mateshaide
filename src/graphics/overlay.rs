use crate::core::color::Color;
use crate::shape::instance::ShapeInstance;
use crate::theme::palette;

pub(super) const OVERLAY_CAP: usize = 8;

const SWATCH_PX: f32 = 48.0;
const SWATCH_BORDER_PX: f32 = 2.0;
const SWATCH_MARGIN_PX: f32 = 16.0;

pub(super) fn build_swatch_instances(color: Color, width: u32, height: u32) -> Vec<ShapeInstance> {
    let width = width.max(1) as f32;
    let height = height.max(1) as f32;

    let outer_half_px = SWATCH_PX * 0.5 + SWATCH_BORDER_PX;
    let inner_half_px = SWATCH_PX * 0.5;

    let outer_half_x = 2.0 * outer_half_px / width;
    let outer_half_y = 2.0 * outer_half_px / height;
    let inner_half_x = 2.0 * inner_half_px / width;
    let inner_half_y = 2.0 * inner_half_px / height;

    let center_x_px = width - SWATCH_MARGIN_PX - outer_half_px;
    let center_y_px = SWATCH_MARGIN_PX + outer_half_px;

    let center_x = 2.0 * center_x_px / width - 1.0;
    let center_y = 1.0 - 2.0 * center_y_px / height;

    vec![
        ShapeInstance::rect(
            center_x - outer_half_x,
            center_y - outer_half_y,
            outer_half_x * 2.0,
            outer_half_y * 2.0,
            palette::GRAY_700,
        ),
        ShapeInstance::rect(
            center_x - inner_half_x,
            center_y - inner_half_y,
            inner_half_x * 2.0,
            inner_half_y * 2.0,
            color,
        ),
    ]
}
