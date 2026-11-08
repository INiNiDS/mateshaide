use crate::core::Color;
use glam::Vec2;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CanvasText {
    pub position: Vec2, // top left corner
    pub text: String,
    pub size: f32, // in world units
    pub color: Color,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font: Option<String>,
}
