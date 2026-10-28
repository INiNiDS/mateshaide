use crate::dev::spawner::TestShapeSpec;
use crate::shape::{KIND_CIRCLE, KIND_RECT};
use crate::{Canvas, Color, ShapeInstance, ShapeRegistry};
use glam::Vec2;

pub(super) const MIN_SIZE: f32 = 8.0;
pub(super) const MAX_SIZE: f32 = 10_000.0;
const COS_30: f32 = 0.866_025_4;

#[derive(Debug, Clone, Copy)]
pub(super) struct Corner(pub Vec2);

pub(super) const CORNERS: [Corner; 4] = [
    Corner(Vec2::new(-1.0, -1.0)),
    Corner(Vec2::new(1.0, -1.0)),
    Corner(Vec2::new(1.0, 1.0)),
    Corner(Vec2::new(-1.0, 1.0)),
];

impl Corner {
    pub(super) fn position(self, shape: &ShapeInstance) -> Vec2 {
        Vec2::from_array(shape.center) + self.0 * Vec2::from_array(shape.size) * 0.5
    }
}

pub(super) fn clamp_size(size: Vec2) -> Vec2 {
    size.clamp(Vec2::splat(MIN_SIZE), Vec2::splat(MAX_SIZE))
}

pub(super) fn build_instance(
    spec: TestShapeSpec,
    center: Vec2,
    requested_size: Vec2,
    color: Color,
) -> ShapeInstance {
    let mut size = clamp_size(requested_size);
    let a = match spec.name {
        "circle" => {
            size = Vec2::splat(size.min_element());
            [size.x * 0.5, 0.0, 0.0, 0.0]
        }
        "hexagon" => {
            let h = size.y.min(size.x * COS_30).max(MIN_SIZE);
            size = Vec2::new(h / COS_30, h);
            [h * 0.5, 0.0, 0.0, 0.0]
        }
        "rounded_rect" => {
            let radius = 12.0_f32.min(size.min_element() * 0.5);
            rounded_params(size, radius)
        }
        _ => [size.x * 0.5, size.y * 0.5, 0.0, 0.0],
    };
    ShapeInstance::new(spec.kind, center, size, color).with_params(a, [0.0; 4])
}

fn rounded_params(size: Vec2, radius: f32) -> [f32; 4] {
    [size.x * 0.5 - radius, size.y * 0.5 - radius, radius, 0.0]
}

pub(super) fn can_resize(shape: &ShapeInstance, registry: &ShapeRegistry) -> bool {
    let size = Vec2::from_array(shape.size);
    size.is_finite()
        && size.min_element() > 0.0
        && size.max_element() / size.min_element() <= MAX_SIZE / MIN_SIZE
        && (shape.kind == KIND_RECT
            || shape.kind == KIND_CIRCLE
            || registry.kind("hexagon") == Some(shape.kind)
            || registry.kind("rounded_rect") == Some(shape.kind))
}

pub(super) fn needs_uniform_resize(shape: &ShapeInstance, registry: &ShapeRegistry) -> bool {
    shape.kind == KIND_CIRCLE || registry.kind("hexagon") == Some(shape.kind)
}

pub(super) fn resize_instance(
    original: ShapeInstance,
    size: Vec2,
    registry: &ShapeRegistry,
) -> ShapeInstance {
    let mut result = original;
    result.size = size.to_array();
    if original.kind == KIND_RECT {
        result.a[0] = size.x * 0.5;
        result.a[1] = size.y * 0.5;
    } else if original.kind == KIND_CIRCLE {
        result.a[0] = size.x * 0.5;
    } else if registry.kind("hexagon") == Some(original.kind) {
        result.a[0] = size.y * 0.5;
    } else if registry.kind("rounded_rect") == Some(original.kind) {
        let scale = (size / Vec2::from_array(original.size)).min_element();
        let radius = (original.a[2] * scale).clamp(0.0, size.min_element() * 0.5);
        result.a = rounded_params(size, radius);
    }
    result
}

pub(super) fn contains(shape: &ShapeInstance, world: Vec2, registry: &ShapeRegistry) -> bool {
    let p = world - Vec2::from_array(shape.center);
    let half = Vec2::from_array(shape.size) / 2.0;
    if p.x.abs() > half.x || p.y.abs() > half.y {
        return false;
    }
    if registry.kind("rounded_rect") == Some(shape.kind) {
        let q = p.abs() - Vec2::new(shape.a[0], shape.a[1]);
        return q.max(Vec2::ZERO).length() + q.max_element().min(0.0) - shape.a[2] <= 0.0;
    }
    if registry.kind("hexagon") == Some(shape.kind) {
        let k = Vec2::new(-COS_30, 0.5);
        let mut q = p.abs();
        q -= 2.0 * k.dot(q).min(0.0) * k;
        q -= Vec2::new(
            q.x.clamp(-0.577_350_26 * shape.a[0], 0.577_350_26 * shape.a[0]),
            shape.a[0],
        );
        return q.y <= 0.0;
    }
    shape.contains_world(world)
}

pub(super) fn pick(canvas: &mut Canvas, world: Vec2, registry: &ShapeRegistry) -> Option<usize> {
    canvas
        .instances
        .iter()
        .rposition(|shape| contains(shape, world, registry))
}

pub(super) fn resize_from_corner(
    original: ShapeInstance,
    corner: Corner,
    moving_corner: Vec2,
    keep_aspect: bool,
    registry: &ShapeRegistry,
) -> ShapeInstance {
    let original_size = Vec2::from_array(original.size);
    let anchor = Vec2::from_array(original.center) - corner.0 * original_size * 0.5;
    let requested = (moving_corner - anchor) * corner.0;
    let size = if keep_aspect || needs_uniform_resize(&original, registry) {
        let factor = requested.dot(original_size) / original_size.length_squared();
        let maximum = (Vec2::splat(MAX_SIZE) / original_size).min_element();
        let minimum = (Vec2::splat(MIN_SIZE) / original_size).max_element();
        original_size * factor.clamp(minimum, maximum)
    } else {
        clamp_size(requested)
    };
    let mut result = resize_instance(original, size, registry);
    result.center = (anchor + corner.0 * size * 0.5).to_array();
    result
}
