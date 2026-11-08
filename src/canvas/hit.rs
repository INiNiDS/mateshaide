use super::model::Canvas;
use crate::export::bounds::{Rect, text_rect};
use crate::shape::{KIND_LINE, ShapeInstance, ShapeRegistry};
use glam::Vec2;

const COS_30: f32 = 0.866_025_4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hit {
    Node(usize),
    Text(usize),
    Shape(usize),
}

#[must_use]
pub fn shape_contains(shape: &ShapeInstance, world: Vec2, registry: &ShapeRegistry) -> bool {
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

#[must_use]
pub fn shape_at(canvas: &Canvas, world: Vec2, registry: &ShapeRegistry) -> Option<usize> {
    canvas
        .instances
        .iter()
        .rposition(|shape| shape_contains(shape, world, registry))
}

fn line_distance(shape: &ShapeInstance, p: Vec2) -> f32 {
    let center = Vec2::from_array(shape.center);
    let a = Vec2::new(shape.a[0], shape.a[1]) + center;
    let b = Vec2::new(shape.a[2], shape.a[3]) + center;
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.length_squared().max(1e-8)).clamp(0.0, 1.0);
    (p - (a + ab * t)).length() - shape.b[0]
}

pub(crate) fn shape_hit(shape: &ShapeInstance, world: Vec2, registry: &ShapeRegistry, tolerance: f32) -> bool {
    if shape.kind == KIND_LINE {
        return line_distance(shape, world) <= tolerance;
    }
    shape_contains(shape, world, registry)
}

fn rect_hit(r: Rect, world: Vec2, tolerance: f32) -> bool {
    world.x >= r.min.x - tolerance
        && world.x <= r.max.x + tolerance
        && world.y >= r.min.y - tolerance
        && world.y <= r.max.y + tolerance
}

fn node_rect(node: &crate::plugins::PluginNode) -> Rect {
    Rect {
        min: Vec2::new(node.x, node.y - node.height),
        max: Vec2::new(node.x + node.width, node.y),
    }
}

#[must_use]
pub fn hit_at(
    canvas: &Canvas,
    world: Vec2,
    registry: &ShapeRegistry,
    tolerance: f32,
) -> Option<Hit> {
    if let Some(i) = canvas
        .plugin_nodes
        .iter()
        .rposition(|n| rect_hit(node_rect(n), world, tolerance))
    {
        return Some(Hit::Node(i));
    }
    if let Some(i) = canvas
        .texts
        .iter()
        .rposition(|t| rect_hit(text_rect(t), world, tolerance))
    {
        return Some(Hit::Text(i));
    }
    canvas
        .instances
        .iter()
        .rposition(|s| shape_hit(s, world, registry, tolerance))
        .map(Hit::Shape)
}

#[must_use]
pub fn hit_bounds(canvas: &Canvas, hit: Hit) -> Option<Rect> {
    match hit {
        Hit::Node(i) => canvas.plugin_nodes.get(i).map(node_rect),
        Hit::Text(i) => canvas.texts.get(i).map(text_rect),
        Hit::Shape(i) => canvas.instances.get(i).map(|s| {
            let center = Vec2::from_array(s.center);
            let half = Vec2::from_array(s.size) * 0.5;
            Rect {
                min: center - half,
                max: center + half,
            }
        }),
    }
}
