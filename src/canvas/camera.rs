use glam::{Mat4, Vec2, Vec4};

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    pub view_proj: Mat4,
    pub inverse_view_proj: Mat4,
    pub params: Vec4,
}

#[derive(Debug, Copy, Clone)]
pub struct Camera {
    pub offset: Vec2,
    pub zoom: f32,
    pub screen_size: Vec2,
}

impl Camera {
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            offset: Vec2::ZERO,
            zoom: 1.0,
            screen_size: Vec2::new(width.max(1.0), height.max(1.0)),
        }
    }

    pub fn resize(&mut self, width: f32, height: f32) {
        self.screen_size = Vec2::new(width.max(1.0), height.max(1.0));
    }

    pub fn build_view_proj(&self) -> Mat4 {
        let half_w = (self.screen_size.x * 0.5) / self.zoom;
        let half_h = (self.screen_size.y * 0.5) / self.zoom;

        let left = self.offset.x - half_w;
        let right = self.offset.x + half_w;
        let bottom = self.offset.y - half_h;
        let top = self.offset.y + half_h;

        glam::camera::rh::proj::directx::orthographic(left, right, bottom, top, -1.0, 1.0)
    }

    pub fn screen_to_world(&self, screen_pos: Vec2) -> Vec2 {
        let center_offset = screen_pos - self.screen_size * 0.5;
        Vec2::new(
            self.offset.x + center_offset.x / self.zoom,
            self.offset.y - center_offset.y / self.zoom,
        )
    }

    pub fn world_to_screen(&self, world: Vec2) -> Vec2 {
        let d = world - self.offset;
        Vec2::new(
            d.x * self.zoom + self.screen_size.x / 2.0,
            -d.y * self.zoom + self.screen_size.y / 2.0,
        )
    }

    pub fn zoom_at(&mut self, cursor_pos: Vec2, delta: f32) {
        let old_world = self.screen_to_world(cursor_pos);

        let zoom_factor = 1.1_f32.powf(delta);
        self.zoom = (self.zoom * zoom_factor).clamp(0.02, 100.0);

        let new_world = self.screen_to_world(cursor_pos);
        self.offset += old_world - new_world;
    }

    pub fn uniform(&self) -> CameraUniform {
        let view_proj = self.build_view_proj();
        CameraUniform {
            view_proj,
            inverse_view_proj: view_proj.inverse(),
            params: Vec4::new(self.zoom, 0.0, 0.0, 0.0),
        }
    }
}
