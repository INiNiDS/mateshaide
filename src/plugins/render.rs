use crate::plugins::UiAction;
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
    #[must_use]
    pub fn len(&self) -> usize {
        self.shapes.len() + self.texts.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.shapes.is_empty() && self.texts.is_empty()
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
        action: UiAction,
        selected: bool,
    ) {
        let background = if selected {
            Color::rgb(0.18, 0.32, 0.72)
        } else {
            match &action {
                UiAction::TogglePage => Color::rgb(0.47, 0.32, 0.86),
                UiAction::Save => Color::rgb(0.12, 0.58, 0.36),
                UiAction::Open => Color::rgb(0.08, 0.49, 0.72),
                UiAction::Tool(_, _) => Color::rgb(0.22, 0.46, 0.86),
                UiAction::Command(_, _) => Color::rgb(0.82, 0.39, 0.12),
                UiAction::TogglePlugin(_) | UiAction::Confirm => Color::rgb(0.12, 0.55, 0.38),
                UiAction::Cancel => Color::rgb(0.48, 0.52, 0.60),
                _ => Color::rgb(0.32, 0.43, 0.64),
            }
        };

        self.rect(r, background, camera.screen_size.x, camera.screen_size.y);

        let capacity = ((r[2] - r[0] - 16.0) / 8.0).max(1.0) as usize;
        let label = if label.chars().count() > capacity {
            let prefix: String = label.chars().take(capacity.saturating_sub(3)).collect();
            format!("{prefix}...")
        } else {
            label.to_owned()
        };

        self.text(camera, r[0] + 8.0, r[1] + 6.0, 14.0, label, Color::WHITE);
        self.hits.push((r, action));
    }

    #[must_use]
    pub fn hit(&self, p: Vec2) -> Option<UiAction> {
        self.hits
            .iter()
            .rev()
            .find(|(r, _)| p.x >= r[0] && p.y >= r[1] && p.x <= r[2] && p.y <= r[3])
            .map(|(_, a)| a.clone())
    }

    pub fn append_clipped(&mut self, layer: Self, camera: &crate::Camera, clip: [f32; 4]) {
        let w = camera.screen_size.x;
        let h = camera.screen_size.y;

        for shape in layer.shapes {
            let left = (shape.center[0] - shape.size[0] * 0.5 + 1.0) * w * 0.5;
            let top = (1.0 - shape.center[1] - shape.size[1] * 0.5) * h * 0.5;

            let r = [
                left.max(clip[0]),
                top.max(clip[1]),
                (left + shape.size[0] * w * 0.5).min(clip[2]),
                (top + shape.size[1] * h * 0.5).min(clip[3]),
            ];

            if r[2] > r[0] && r[3] > r[1] {
                self.rect(r, shape.color, w, h);
            }
        }

        for text in layer.texts {
            let position = camera.world_to_screen(text.position);
            let size = text.size * camera.zoom;
            let line_height = size * 1.25;

            for (index, line) in text.text.split('\n').enumerate() {
                let y = position.y + index as f32 * line_height;

                if position.x >= clip[0]
                    && position.x < clip[2]
                    && y >= clip[1]
                    && y + line_height <= clip[3]
                {
                    self.text(camera, position.x, y, size, line, text.color);
                }
            }
        }

        for (r, action) in layer.hits {
            if r[0] >= clip[0] && r[1] >= clip[1] && r[2] <= clip[2] && r[3] <= clip[3] {
                self.hits.push((r, action));
            }
        }
    }
    #[allow(clippy::too_many_arguments)]
    pub fn button_styled(
        &mut self,
        camera: &crate::Camera,
        r: [f32; 4],
        label: &str,
        action: UiAction,
        selected: bool,
        style: &super::runtime::ButtonStyle,
    ) {
        self.button(camera, r, label, action, selected);

        let background = if selected {
            style.selected_background
        } else {
            style.background
        };

        if let Some([red, green, blue, alpha]) = background {
            if let Some(shape) = self.shapes.last_mut() {
                shape.color = Color::new(red, green, blue, alpha);
            }
        }

        if let Some([red, green, blue, alpha]) = style.text {
            if let Some(text) = self.texts.last_mut() {
                text.color = Color::new(red, green, blue, alpha);
            }
        }
    }
}
