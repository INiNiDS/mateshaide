use crate::core::Color;
use glam::Vec2;

#[derive(Debug, Clone)]
pub struct CanvasText {
    pub position: Vec2, // top left corner
    pub text: String,
    pub size: f32, // in world units
    pub color: Color,
}
