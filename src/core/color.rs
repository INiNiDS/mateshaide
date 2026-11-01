use glam::{Vec3, Vec4};
use std::str::FromStr;

// RGBA
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Default for Color {
    fn default() -> Self {
        Self::BLACK
    }
}

impl Color {
    pub const TRANSPARENT: Self = Self::new(0.0, 0.0, 0.0, 0.0);
    pub const BLACK: Self = Self::rgb(0.0, 0.0, 0.0);
    pub const WHITE: Self = Self::rgb(1.0, 1.0, 1.0);
    pub const RED: Self = Self::rgb(1.0, 0.0, 0.0);
    pub const GREEN: Self = Self::rgb(0.0, 1.0, 0.0);
    pub const BLUE: Self = Self::rgb(0.0, 0.0, 1.0);
    pub const YELLOW: Self = Self::rgb(1.0, 1.0, 0.0);
    pub const CYAN: Self = Self::rgb(0.0, 1.0, 1.0);
    pub const MAGENTA: Self = Self::rgb(1.0, 0.0, 1.0);

    #[must_use]
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    #[must_use]
    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    #[must_use]
    pub const fn hexa(v: u32) -> Self {
        Self::rgba8(
            ((v >> 24) & 0xFF) as u8,
            ((v >> 16) & 0xFF) as u8,
            ((v >> 8) & 0xFF) as u8,
            (v & 0xFF) as u8,
        )
    }

    #[must_use]
    pub const fn hex(v: u32) -> Self {
        Self::rgba8(
            ((v >> 16) & 0xFF) as u8,
            ((v >> 8) & 0xFF) as u8,
            (v & 0xFF) as u8,
            255,
        )
    }

    #[must_use]
    pub const fn rgba8(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self::new(
            (r as f32) / 255.0,
            (g as f32) / 255.0,
            (b as f32) / 255.0,
            a as f32 / 255.0,
        )
    }

    #[must_use]
    pub fn from_hsv(h: f32, s: f32, v: f32) -> Self {
        let h = h.rem_euclid(360.0) / 60.0;
        let c = s * v;
        let x = c * (1.0 - ((h % 2.0) - 1.0).abs());
        let m = v - c;
        let (r, g, b) = match h as u32 {
            0 => (c, x, 0.0),
            1 => (x, c, 0.0),
            2 => (0.0, c, x),
            3 => (0.0, x, c),
            4 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };
        Self::rgb(r + m, g + m, b + m)
    }

    #[must_use]
    pub fn to_array(&self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }

    #[must_use]
    pub fn to_array3(&self) -> [f32; 3] {
        [self.r, self.g, self.b]
    }

    #[must_use]
    pub fn to_vec4(&self) -> Vec4 {
        Vec4::new(self.r, self.g, self.b, self.a)
    }

    #[must_use]
    pub fn to_vec3(&self) -> Vec3 {
        Vec3::new(self.r, self.g, self.b)
    }

    #[must_use]
    pub fn to_wgpu(&self) -> wgpu::Color {
        wgpu::Color {
            r: f64::from(self.r),
            g: f64::from(self.g),
            b: f64::from(self.b),
            a: f64::from(self.a),
        }
    }

    #[must_use]
    pub fn to_linear(&self) -> Self {
        Self::new(
            srgb_to_linear(self.r),
            srgb_to_linear(self.g),
            srgb_to_linear(self.b),
            self.a,
        )
    }

    #[must_use]
    pub fn to_srgb(&self) -> Self {
        Self::new(
            linear_to_srgb(self.r),
            linear_to_srgb(self.g),
            linear_to_srgb(self.b),
            self.a,
        )
    }

    #[must_use]
    pub fn for_surface(self, format: wgpu::TextureFormat) -> Self {
        if format.is_srgb() {
            self.to_linear()
        } else {
            self
        }
    }

    #[must_use]
    pub fn with_alpha(self, a: f32) -> Self {
        Self { a, ..self }
    }

    #[must_use]
    pub fn lerp(self, other: Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        Self::new(
            self.r + (other.r - self.r) * t,
            self.g + (other.g - self.g) * t,
            self.b + (other.b - self.b) * t,
            self.a + (other.a - self.a) * t,
        )
    }

    #[must_use]
    pub fn lighten(self, t: f32) -> Self {
        self.lerp(Self::WHITE, t)
    }

    #[must_use]
    pub fn darken(self, t: f32) -> Self {
        self.lerp(Self::BLACK, t)
    }
}

fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(c: f32) -> f32 {
    if c <= 0.0031308 {
        12.92 * c
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("invalid color: `{0}`")]
pub struct ColorParseError(pub String);

impl FromStr for Color {
    type Err = ColorParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let err = || ColorParseError(s.to_owned());
        let hex = s.strip_prefix("#").unwrap_or(s);
        if !hex.is_ascii() {
            return Err(err());
        }

        let bytes: Vec<u8> = match hex.len() {
            3 | 4 => hex
                .chars()
                .map(|c| c.to_digit(16).map(|d| d as u8 * 17))
                .collect::<Option<Vec<_>>>()
                .ok_or_else(err)?,
            6 | 8 => (0..hex.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).ok())
                .collect::<Option<Vec<_>>>()
                .ok_or_else(err)?,
            _ => return Err(err()),
        };
        let a = bytes.get(3).copied().unwrap_or(255);
        Ok(Self::rgba8(bytes[0], bytes[1], bytes[2], a))
    }
}

impl From<[f32; 4]> for Color {
    fn from(rgba: [f32; 4]) -> Self {
        Self::new(rgba[0], rgba[1], rgba[2], rgba[3])
    }
}

impl From<[f32; 3]> for Color {
    fn from(c: [f32; 3]) -> Self {
        Self::rgb(c[0], c[1], c[2])
    }
}

impl From<Vec4> for Color {
    fn from(v: Vec4) -> Self {
        Self::new(v.x, v.y, v.z, v.w)
    }
}

impl From<Vec3> for Color {
    fn from(v: Vec3) -> Self {
        Self::rgb(v.x, v.y, v.z)
    }
}

impl From<(u8, u8, u8)> for Color {
    fn from((r, g, b): (u8, u8, u8)) -> Self {
        Self::rgba8(r, g, b, 255)
    }
}

/// `0xRRGGBB`.
impl From<u32> for Color {
    fn from(v: u32) -> Self {
        Self::hex(v)
    }
}

impl From<Color> for [f32; 4] {
    fn from(c: Color) -> Self {
        c.to_array()
    }
}

impl From<Color> for [f32; 3] {
    fn from(c: Color) -> Self {
        c.to_array3()
    }
}

impl From<Color> for Vec4 {
    fn from(c: Color) -> Self {
        c.to_vec4()
    }
}

impl From<Color> for Vec3 {
    fn from(c: Color) -> Self {
        c.to_vec3()
    }
}

impl From<Color> for wgpu::Color {
    fn from(c: Color) -> Self {
        c.to_wgpu()
    }
}

impl From<Color> for glyphon::Color {
    fn from(color: Color) -> Self {
        glyphon::Color::rgba(
            (color.r.clamp(0.0, 1.0) * 255.0) as u8,
            (color.g.clamp(0.0, 1.0) * 255.0) as u8,
            (color.b.clamp(0.0, 1.0) * 255.0) as u8,
            (color.a.clamp(0.0, 1.0) * 255.0) as u8,
        )
    }
}
