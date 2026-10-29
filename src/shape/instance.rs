use crate::core::color::Color;
use glam::Vec2;
use std::mem::offset_of;
use wgpu::VertexFormat;

pub const KIND_RECT: u32 = 0;
pub const KIND_CIRCLE: u32 = 1;
pub const KIND_LINE: u32 = 2;

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ShapeInstance {
    pub center: [f32; 2],
    pub size: [f32; 2],
    pub color: Color,
    pub kind: u32,
    pub _pad: [u32; 3],
    pub a: [f32; 4],
    pub b: [f32; 4],
}

impl ShapeInstance {
    pub fn new(kind: u32, center: Vec2, size: Vec2, color: impl Into<Color>) -> Self {
        Self {
            center: center.to_array(),
            size: size.to_array(),
            color: color.into(),
            kind,
            _pad: [0; 3],
            a: [0.0; 4],
            b: [0.0; 4],
        }
    }

    #[must_use]
    pub fn with_params(mut self, a: [f32; 4], b: [f32; 4]) -> Self {
        self.a = a;
        self.b = b;
        self
    }

    pub fn rect(x: f32, y: f32, width: f32, height: f32, color: impl Into<Color>) -> Self {
        let size = Vec2::new(width, height);
        let center = Vec2::new(x, y) + size * 0.5;

        Self::new(KIND_RECT, center, size, color)
            .with_params([width * 0.5, height * 0.5, 0.0, 0.0], [0.0; 4])
    }

    #[must_use]
    pub fn contains_world(&self, p: Vec2) -> bool {
        let center = Vec2::new(self.center[0], self.center[1]);
        let half = Vec2::new(self.size[0], self.size[1]) * 0.5;
        let d = p - center;

        match self.kind {
            KIND_RECT => d.x.abs() <= half.x && d.y.abs() <= half.y,

            KIND_CIRCLE => d.length_squared() <= half.x * half.x,

            KIND_LINE => {
                let a = Vec2::new(self.a[0], self.a[1]) + center;
                let b = Vec2::new(self.a[2], self.a[3]) + center;
                let ab = b - a;

                let t = ((p - a).dot(ab) / ab.length_squared().max(1e-8)).clamp(0.0, 1.0);
                let closest = a + ab * t;

                (p - closest).length() <= self.b[0]
            }

            _ => {
                // Fallback to rect for unknown kinds.
                d.x.abs() <= half.x && d.y.abs() <= half.y
            }
        }
    }

    pub fn circle(x: f32, y: f32, radius: f32, color: impl Into<Color>) -> Self {
        Self::new(
            KIND_CIRCLE,
            Vec2::new(x, y),
            Vec2::splat(radius * 2.0),
            color,
        )
        .with_params([radius, 0.0, 0.0, 0.0], [0.0; 4])
    }

    pub fn line(start: Vec2, end: Vec2, radius: f32, color: impl Into<Color>) -> Self {
        let min = start.min(end) - Vec2::splat(radius);
        let max = start.max(end) + Vec2::splat(radius);

        let center = (min + max) * 0.5;
        let size = max - min;

        let l1 = start - center;
        let l2 = end - center;

        Self::new(KIND_LINE, center, size, color)
            .with_params([l1.x, l1.y, l2.x, l2.y], [radius, 0.0, 0.0, 0.0])
    }

    #[must_use]
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    format: VertexFormat::Float32x2,
                    offset: offset_of!(Self, center) as wgpu::BufferAddress,
                    shader_location: 0,
                },
                wgpu::VertexAttribute {
                    format: VertexFormat::Float32x2,
                    offset: offset_of!(Self, size) as wgpu::BufferAddress,
                    shader_location: 1,
                },
                wgpu::VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: offset_of!(Self, color) as wgpu::BufferAddress,
                    shader_location: 2,
                },
                wgpu::VertexAttribute {
                    format: VertexFormat::Uint32,
                    offset: offset_of!(Self, kind) as wgpu::BufferAddress,
                    shader_location: 3,
                },
                wgpu::VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: offset_of!(Self, a) as wgpu::BufferAddress,
                    shader_location: 4,
                },
                wgpu::VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: offset_of!(Self, b) as wgpu::BufferAddress,
                    shader_location: 5,
                },
            ],
        }
    }
}
