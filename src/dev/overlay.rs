use crate::dev::geometry::CORNERS;
use crate::dev::{TestMode, TestSpawner, geometry};
use crate::{palette, Canvas, CanvasText, Color, ShapeInstance, ShapeRegistry};
use glam::Vec2;

pub(crate) const INSTANCES_CAP: usize = 32;
const HANDLE_PX: f32 = 9.0;

pub(crate) fn build_instances(
    canvas: &Canvas,
    registry: &ShapeRegistry,
    tool: &TestSpawner,
) -> Vec<ShapeInstance> {
    let mut result = Vec::with_capacity(16);
    if let Some(mut preview) = tool.preview_instance(canvas) {
        preview.color = preview.color.with_alpha(preview.color.a * 0.35);
        result.push(preview);
    }
    if tool.mode == Some(TestMode::Select) {
        if let Some(shape) = tool
            .selected()
            .and_then(|index| canvas.instances.get(index))
        {
            let center = Vec2::from_array(shape.center);
            let size = Vec2::from_array(shape.size);
            let min = center - size / 2.0;
            let max = center + size / 2.0;
            let t = 1.5 / canvas.camera.zoom;
            let color = palette::BLUE_500;
            result.extend([
                ShapeInstance::rect(min.x - t, min.y - t, size.x + 2.0 * t, t, color),
                ShapeInstance::rect(min.x - t, max.y, size.x + 2.0 * t, t, color),
                ShapeInstance::rect(min.x - t, min.y, t, size.y, color),
                ShapeInstance::rect(max.x, min.y, t, size.y, color),
            ]);
            if geometry::can_resize(shape, registry) {
                for corner in CORNERS {
                    let p = corner.position(shape);
                    let outer = HANDLE_PX / canvas.camera.zoom;
                    let inner = (HANDLE_PX - 3.0) / canvas.camera.zoom;
                    result.push(centered_rect(p, outer, color));
                    result.push(centered_rect(p, inner, palette::WHITE));
                }
            }
        }
    }
    debug_assert!(result.len() <= INSTANCES_CAP);
    result
}

fn centered_rect(center: Vec2, size: f32, color: Color) -> ShapeInstance {
    ShapeInstance::rect(
        center.x - size * 0.5,
        center.y - size * 0.5,
        size,
        size,
        color,
    )
}

pub(crate) fn build_texts(canvas: &Canvas, tool: &TestSpawner) -> Vec<CanvasText> {
    let Some(mode) = tool.mode else {
        return Vec::new();
    };
    let text = match mode {
        TestMode::Spawn => {
            let Some(preview) = tool.preview_instance(canvas) else {
                return Vec::new();
            };
            let name = tool.current.map_or("shape", |spec| spec.name);
            format!(
                "DEV | spawn {name} | {:.1} x {:.1} | click / drag | [ ] size",
                preview.size[0], preview.size[1]
            )
        }
        TestMode::Select => match tool
            .selected()
            .and_then(|index| canvas.instances.get(index))
        {
            Some(shape) => format!(
                "DEV | select | {:.1} x {:.1} | drag to move | corners to resize | Shift: ratio",
                shape.size[0], shape.size[1]
            ),
            None => "DEV | select | click a shape | V: leave selection mode".to_owned(),
        },
        TestMode::Erase => "DEV | eraser | click / 1: remove topmost shape".to_owned(),
    };

    vec![CanvasText {
        position: canvas.camera.screen_to_world(Vec2::new(16.0, 16.0)),
        size: 14.0 / canvas.camera.zoom,
        text,
        color: palette::GRAY_900
    }]
}
