use crate::{Camera, CanvasText, ShapeInstance};
use glam::Vec2;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub min: Vec2,
    pub max: Vec2,
}

impl Rect {
    #[must_use]
    pub fn size(&self) -> Vec2 {
        self.max - self.min
    }

    #[must_use]
    pub fn center(&self) -> Vec2 {
        (self.min + self.max) / 2.0
    }

    #[must_use]
    pub fn union(self, other: Rect) -> Rect {
        Rect {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    #[must_use]
    pub fn padded(self, amount: f32) -> Rect {
        Rect {
            min: self.min - Vec2::splat(amount),
            max: self.max + Vec2::splat(amount),
        }
    }

    #[must_use]
    pub fn intersects(self, other: Rect) -> bool {
        self.min.x < other.max.x
            && self.max.x > other.min.x
            && self.min.y < other.max.y
            && self.max.y > other.min.y
    }
}

fn shape_rect(shape: &ShapeInstance) -> Rect {
    let center = Vec2::from(shape.center);
    let half = Vec2::from(shape.size) * 0.5;
    Rect {
        min: center - half,
        max: center + half,
    }
}

fn text_rect(text: &CanvasText) -> Rect {
    let width = text.text.chars().count() as f32 * text.size * 0.6;
    let height = text.size * 1.25;

    Rect {
        min: Vec2::new(text.position.x, text.position.y - height),
        max: Vec2::new(text.position.x + width, text.position.y),
    }
}

#[must_use]
pub fn item_rects(instances: &[ShapeInstance], texts: &[CanvasText]) -> Vec<Rect> {
    instances
        .iter()
        .map(shape_rect)
        .chain(texts.iter().map(text_rect))
        .collect()
}

#[must_use]
pub fn content_bounds(rects: &[Rect]) -> Option<Rect> {
    rects.iter().copied().reduce(Rect::union)
}

#[must_use]
pub fn frame_camera(rect: Rect, px_w: u32, px_h: u32) -> Camera {
    let mut camera = Camera::new(px_w as f32, px_h as f32);
    let size = rect.size();
    camera.zoom = (px_w as f32 / size.x).min(px_h as f32 / size.y);
    camera.offset = rect.center();
    camera
}