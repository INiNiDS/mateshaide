use crate::{CanvasText, Color, ShapeInstance};
use anyhow::{Context, Result, bail, ensure};
use glam::Vec2;
use serde_json::Value;

#[derive(Default, Clone)]
pub struct Scene {
    pub shapes: Vec<ShapeInstance>,
    pub texts: Vec<CanvasText>,
}
impl Scene {
    pub fn len(&self) -> usize {
        self.shapes.len() + self.texts.len()
    }
    pub fn extend_primitives(&mut self, value: &Value) -> Result<()> {
        let items = value.as_array().context("render must return an array")?;
        ensure!(
            items.len() <= 4096 && self.len() + items.len() <= 65536,
            "primitive limit"
        );
        let mut shapes = Vec::new();
        let mut texts = Vec::new();
        for p in items {
            let c = color(p.get("color"))?;
            match p["kind"].as_str().context("missing primitive kind")? {
                "text" => {
                    let text = p["text"].as_str().context("missing text")?;
                    ensure!(text.len() <= 32768, "render text limit");
                    let size = num(p, "size")?;
                    ensure!((4.0..=512.0).contains(&size), "font size limit");
                    texts.push(CanvasText {
                        position: Vec2::new(num(p, "x")?, num(p, "y")?),
                        text: text.into(),
                        size,
                        color: c,
                    });
                }
                "rect" => {
                    let w = num(p, "width")?;
                    let h = num(p, "height")?;
                    ensure!(w >= 0.0 && h >= 0.0, "negative rectangle");
                    shapes.push(ShapeInstance::rect(num(p, "x")?, num(p, "y")? - h, w, h, c));
                }
                "circle" => {
                    let r = num(p, "radius")?;
                    ensure!(r >= 0.0, "negative radius");
                    shapes.push(ShapeInstance::circle(num(p, "x")?, num(p, "y")?, r, c));
                }
                "line" => {
                    let width = num(p, "width")?;
                    ensure!(width >= 0.0, "negative line width");
                    shapes.push(ShapeInstance::line(
                        Vec2::new(num(p, "x1")?, num(p, "y1")?),
                        Vec2::new(num(p, "x2")?, num(p, "y2")?),
                        width * 0.5,
                        c,
                    ));
                }
                _ => bail!("unknown graphics primitive"),
            }
        }
        self.shapes.extend(shapes);
        self.texts.extend(texts);
        Ok(())
    }
    pub fn placeholder(&mut self, n: &super::PluginNode) {
        self.shapes.push(ShapeInstance::rect(
            n.x,
            n.y - n.height.max(40.0),
            n.width.max(160.0),
            n.height.max(40.0),
            Color::new(0.7, 0.35, 0.1, 0.22),
        ));
        self.texts.push(CanvasText {
            position: Vec2::new(n.x + 4.0, n.y - 4.0),
            text: format!("{} (unavailable)", n.owner),
            size: 12.0,
            color: Color::rgb(0.9, 0.55, 0.2),
        });
    }
}
pub fn num(p: &Value, key: &str) -> Result<f32> {
    let n = p[key]
        .as_f64()
        .with_context(|| format!("missing number: {key}"))?;
    ensure!(n.is_finite() && n.abs() <= 1.0e8, "invalid number");
    Ok(n as f32)
}

fn color(v: Option<&Value>) -> Result<Color> {
    let Some(v) = v else { return Ok(Color::WHITE) };
    let a = v.as_array().context("color must be RGBA array")?;
    ensure!(a.len() == 4, "RGBA needs four values");
    let mut c = [0.0f32; 4];
    for (i, n) in a.iter().enumerate() {
        let n = n.as_f64().context("invalid color")?;
        ensure!(
            n.is_finite() && (0.0..=1.0).contains(&n),
            "color out of range"
        );
        c[i] = n as f32;
    }
    Ok(Color::new(c[0], c[1], c[2], c[3]))
}

#[derive(Default)]
pub struct UiFrame {
    pub shapes: Vec<ShapeInstance>,
    pub texts: Vec<CanvasText>,
    pub hits: Vec<([f32; 4], super::ui::UiAction)>,
}
impl UiFrame {
    pub fn rect(&mut self, screen: [f32; 4], color: Color, w: f32, h: f32) {
        let [x, y, r, b] = screen;
        self.shapes.push(ShapeInstance::rect(
            2.0 * x / w - 1.0,
            1.0 - 2.0 * b / h,
            2.0 * (r - x) / w,
            2.0 * (b - y) / h,
            color,
        ));
    }
    pub fn text(
        &mut self,
        camera: &crate::Camera,
        x: f32,
        y: f32,
        size: f32,
        text: impl Into<String>,
        color: Color,
    ) {
        self.texts.push(CanvasText {
            position: camera.screen_to_world(Vec2::new(x, y)),
            text: text.into(),
            size: size / camera.zoom,
            color,
        });
    }
    pub fn button(
        &mut self,
        camera: &crate::Camera,
        r: [f32; 4],
        label: &str,
        action: super::ui::UiAction,
        selected: bool,
    ) {
        self.rect(
            r,
            if selected {
                Color::rgb(0.19, 0.36, 0.55)
            } else {
                Color::rgb(0.16, 0.19, 0.24)
            },
            camera.screen_size.x,
            camera.screen_size.y,
        );
        self.text(camera, r[0] + 8.0, r[1] + 6.0, 14.0, label, Color::WHITE);
        self.hits.push((r, action));
    }
    pub fn hit(&self, p: Vec2) -> Option<super::ui::UiAction> {
        self.hits
            .iter()
            .rev()
            .find(|(r, _)| p.x >= r[0] && p.y >= r[1] && p.x <= r[2] && p.y <= r[3])
            .map(|(_, a)| a.clone())
    }
}
