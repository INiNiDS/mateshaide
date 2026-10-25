use glam::Vec2;

#[derive(Debug, Clone)]
pub enum Sdf {
    Prim { name: String, params: [[f32; 4]; 2] },
    Translate(Box<Sdf>, Vec2),
    Union(Vec<Sdf>),
    Subtract(Box<Sdf>, Box<Sdf>),
    SmoothUnion(Box<Sdf>, Box<Sdf>, f32),
}
