use crate::color::Color;
use crate::utils::{FastHashMap, FastHashSet};
use glam::Vec2;
use std::fmt::Write;
use std::mem::offset_of;
use wgpu::VertexFormat;

pub const KIND_RECT: u32 = 0;
pub const KIND_CIRCLE: u32 = 1;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
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

    pub fn circle(x: f32, y: f32, radius: f32, color: impl Into<Color>) -> Self {
        Self::new(
            KIND_CIRCLE,
            Vec2::new(x, y),
            Vec2::splat(radius * 2.0),
            color,
        )
        .with_params([radius, 0.0, 0.0, 0.0], [0.0; 4])
    }

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

#[derive(Debug, Clone)]
pub enum Sdf {
    Prim { name: String, params: [[f32; 4]; 2] },
    Translate(Box<Sdf>, Vec2),
    Union(Vec<Sdf>),
    Subtract(Box<Sdf>, Box<Sdf>),
    SmoothUnion(Box<Sdf>, Box<Sdf>, f32),
}

#[derive(Default, Clone)]
pub struct ShapeRegistry {
    wgsl: String,
    dispatch: Vec<String>,
    kinds: FastHashMap<String, u32>,
    prims: FastHashSet<String>,
    counter: u32,
    version: u64,
}

impl ShapeRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_builtins() -> Result<Self, ShapeError> {
        let mut reg = Self::new();
        let rect = reg.define_primitive("rect", "sd_box(p, a.xy)")?;
        let circle = reg.define_primitive("circle", "length(p) - a.x")?;
        debug_assert_eq!((rect, circle), (KIND_RECT, KIND_CIRCLE));

        reg.define_primitive("hexagon", "sd_hexagon(p, a.x)")?;
        reg.define_primitive("rounded_rect", "sd_box(p, a.xy) - a.z")?;

        Ok(reg)
    }

    pub fn define_primitive(&mut self, name: &str, expr: &str) -> Result<u32, ShapeError> {
        if self.kinds.contains_key(name) {
            return Err(ShapeError::AlreadyDefined(name.to_owned()));
        }
        let fn_name = format!("shape_{name}");
        writeln!(
            self.wgsl,
            "fn {fn_name}(p: vec2<f32>, a: vec4<f32>, b: vec4<f32>) -> f32 {{ return {expr}; }}"
        )?;
        self.prims.insert(name.to_owned());
        Ok(self.push_kind(name, fn_name))
    }

    pub fn define_composite(&mut self, name: &str, root: &Sdf) -> Result<u32, ShapeError> {
        if self.kinds.contains_key(name) {
            return Err(ShapeError::AlreadyDefined(name.to_owned()));
        }
        let root_fn = self.emit(root)?;
        let fn_name = format!("shape_{name}");
        writeln!(
            self.wgsl,
            "fn {fn_name}(p: vec2<f32>, a: vec4<f32>, b: vec4<f32>) -> f32 {{ return {root_fn}(p); }}"
        )?;
        // Композит тоже можно использовать как примитив в другом композите
        self.prims.insert(name.to_owned());
        Ok(self.push_kind(name, fn_name))
    }

    pub fn kind(&self, name: &str) -> Option<u32> {
        self.kinds.get(name).copied()
    }

    pub fn version(&self) -> u64 {
        self.version
    }

    pub fn build_wgsl(&self) -> String {
        let mut s = String::from(include_str!("shape_base.wgsl"));
        s += &self.wgsl;
        s += "fn eval_shape(kind: u32, p: vec2<f32>, a: vec4<f32>, b: vec4<f32>) -> f32 {\n";
        s += "    var d: f32 = 1e9;\n    switch kind {\n";
        for (id, fn_name) in self.dispatch.iter().enumerate() {
            let _ = writeln!(s, "        case {id}u: {{ d = {fn_name}(p, a, b); }}");
        }
        s += "        default: {}\n    }\n    return d;\n}\n";
        s
    }

    fn push_kind(&mut self, name: &str, fn_name: String) -> u32 {
        let id = self.dispatch.len() as u32;
        self.dispatch.push(fn_name);
        self.kinds.insert(name.to_owned(), id);
        self.version += 1;
        id
    }

    fn emit(&mut self, node: &Sdf) -> Result<String, ShapeError> {
        self.counter += 1;
        let fn_name = format!("node_{}", self.counter);

        let body = match node {
            Sdf::Prim { name, params } => {
                if !self.prims.contains(name) {
                    return Err(ShapeError::UnknownPrimitive(name.clone()));
                }
                format!(
                    "return shape_{name}(p, {}, {});",
                    vec4_lit(params[0]),
                    vec4_lit(params[1])
                )
            }
            Sdf::Translate(child, off) => {
                let f = self.emit(child)?;
                format!(
                    "return {f}(p - vec2<f32>({}, {}));",
                    f32_lit(off.x),
                    f32_lit(off.y)
                )
            }
            Sdf::Union(children) => {
                let fs = children
                    .iter()
                    .map(|c| self.emit(c))
                    .collect::<Result<Vec<_>, ShapeError>>()?;
                match fs.split_first() {
                    None => "return 1e9;".to_owned(),
                    Some((first, rest)) => {
                        let expr = rest.iter().fold(format!("{first}(p)"), |acc, f| {
                            format!("min({acc}, {f}(p))")
                        });
                        format!("return {expr};")
                    }
                }
            }
            Sdf::Subtract(a, b) => {
                let fa = self.emit(a)?;
                let fb = self.emit(b)?;
                format!("return max({fa}(p), -{fb}(p));")
            }
            Sdf::SmoothUnion(a, b, k) => {
                let fa = self.emit(a)?;
                let fb = self.emit(b)?;
                format!("return smin({fa}(p), {fb}(p), {});", f32_lit(*k))
            }
        };

        writeln!(self.wgsl, "fn {fn_name}(p: vec2<f32>) -> f32 {{ {body} }}")?;
        Ok(fn_name)
    }
}

fn f32_lit(v: f32) -> String {
    format!("{v:?}")
}

fn vec4_lit(v: [f32; 4]) -> String {
    format!(
        "vec4<f32>({}, {}, {}, {})",
        f32_lit(v[0]),
        f32_lit(v[1]),
        f32_lit(v[2]),
        f32_lit(v[3])
    )
}

#[derive(Debug, thiserror::Error)]
pub enum ShapeError {
    #[error("shape `{0}` already defined")]
    AlreadyDefined(String),

    #[error("unknown primitive `{0}`")]
    UnknownPrimitive(String),

    #[error("failed to generate WGSL")]
    Format(#[from] std::fmt::Error),

    #[error(transparent)]
    Gpu(#[from] wgpu::Error),
}
