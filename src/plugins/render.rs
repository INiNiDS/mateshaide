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

                    let font = match p.get("font") {
                        None | Some(Value::Null) => None,
                        Some(v) => {
                            let name = v.as_str().context("font must be a string")?;
                            ensure!(name.len() <= 64, "font name limit");
                            Some(name.to_owned())
                        }
                    };
                    texts.push(CanvasText {
                        position: Vec2::new(num(p, "x")?, num(p, "y")?),
                        text: text.into(),
                        size,
                        color: c,
                        font,
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
            font: None,
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
    let Some(v) = v else {
        return Ok(Color::WHITE);
    };

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

#[derive(Clone, Copy)]
#[repr(usize)]
pub(crate) enum UiImageKind {
    Blood = 0,
    Pumpkin = 1,
    Strips = 2,
    Web = 3,
    Mascot = 4,
    CornerWeb = 5,
    Eyes = 6,
    Spider = 7,
}

impl UiImageKind {
    pub(crate) const COUNT: usize = 8;
}

pub(crate) struct UiImage {
    pub kind: UiImageKind,
    pub bounds: [f32; 4],
    pub anchor: [f32; 2],
    pub flip_x: bool,
    pub after_shapes: usize,
}

#[derive(Default)]
pub struct UiFrame {
    pub theme: crate::theme::Theme,
    pub shapes: Vec<ShapeInstance>,
    pub texts: Vec<CanvasText>,
    pub hits: Vec<([f32; 4], UiAction)>,
    pub(crate) images: Vec<UiImage>,
    pub(crate) background_shape_count: usize,
    pub(crate) pointer_regions: Vec<[f32; 4]>,
}

impl UiFrame {
    #[must_use]
    pub fn with_theme(theme: crate::theme::Theme) -> Self {
        Self {
            theme,
            ..Self::default()
        }
    }

    pub(crate) fn image(
        &mut self,
        kind: UiImageKind,
        bounds: [f32; 4],
        anchor: [f32; 2],
        flip_x: bool,
    ) {
        self.images.push(UiImage {
            kind,
            bounds,
            anchor,
            flip_x,
            after_shapes: self.shapes.len(),
        });
    }

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

    pub fn rounded_rect(&mut self, r: [f32; 4], radius: f32, color: Color, w: f32, h: f32) {
        let [left, top, right, bottom] = r;

        if right <= left || bottom <= top {
            return;
        }

        let radius = radius
            .min((right - left) * 0.5)
            .min((bottom - top) * 0.5)
            .max(0.0);

        if radius < 1.0 {
            self.rect(r, color, w, h);
            return;
        }

        self.rect([left, top + radius, right, bottom - radius], color, w, h);

        let rows = radius.ceil() as usize;

        for row in 0..rows {
            let y0 = row as f32;
            let y1 = (y0 + 1.0).min(radius);
            let dy = radius - f32::midpoint(y0, y1);
            let inset = radius - (radius * radius - dy * dy).max(0.0).sqrt();

            self.rect(
                [left + inset, top + y0, right - inset, top + y1],
                color,
                w,
                h,
            );

            self.rect(
                [left + inset, bottom - y1, right - inset, bottom - y0],
                color,
                w,
                h,
            );
        }
    }

    pub fn floating_panel(&mut self, camera: &crate::Camera, r: [f32; 4]) {
        let w = camera.screen_size.x;
        let h = camera.screen_size.y;

        self.rounded_rect(
            [r[0], r[1] + 4.0, r[2], r[3] + 4.0],
            12.0,
            Color::new(0.0, 0.0, 0.0, 0.18),
            w,
            h,
        );

        self.rounded_rect(r, 12.0, self.theme.border, w, h);

        self.rounded_rect(
            [r[0] + 1.0, r[1] + 1.0, r[2] - 1.0, r[3] - 1.0],
            11.0,
            self.theme.panel,
            w,
            h,
        );

        self.pointer_regions.push(r);
    }

    #[must_use]
    pub fn blocks_pointer(&self, p: Vec2) -> bool {
        self.pointer_regions
            .iter()
            .any(|r| p.x >= r[0] && p.y >= r[1] && p.x <= r[2] && p.y <= r[3])
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
            font: None,
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
        self.button_styled(
            camera,
            r,
            label,
            action,
            selected,
            &super::runtime::ButtonStyle::default(),
        );
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
        let Self {
            shapes,
            texts,
            hits,
            pointer_regions,
            ..
        } = layer;

        self.append_clipped_shapes(&shapes, camera, clip);
        self.append_clipped_texts(&texts, camera, clip);
        self.append_clipped_regions(&pointer_regions, clip);
        self.append_clipped_hits(hits, clip);
    }

    fn append_clipped_shapes(
        &mut self,
        shapes: &[ShapeInstance],
        camera: &crate::Camera,
        clip: [f32; 4],
    ) {
        let w = camera.screen_size.x;
        let h = camera.screen_size.y;

        for shape in shapes {
            if let Some(r) = intersect(shape_screen_bounds(shape, w, h), clip) {
                self.rect(r, shape.color, w, h);
            }
        }
    }

    fn append_clipped_texts(
        &mut self,
        texts: &[CanvasText],
        camera: &crate::Camera,
        clip: [f32; 4],
    ) {
        for text in texts {
            let position = camera.world_to_screen(text.position);
            let size = text.size * camera.zoom;
            let line_height = size * 1.25;

            for (index, line) in text.text.split('\n').enumerate() {
                let y = position.y + index as f32 * line_height;

                if line_inside_clip(position.x, y, line_height, clip) {
                    self.text(camera, position.x, y, size, line, text.color);
                }
            }
        }
    }

    fn append_clipped_regions(&mut self, regions: &[[f32; 4]], clip: [f32; 4]) {
        for &region in regions {
            if let Some(clipped) = intersect(region, clip) {
                self.pointer_regions.push(clipped);
            }
        }
    }

    fn append_clipped_hits(&mut self, hits: Vec<([f32; 4], UiAction)>, clip: [f32; 4]) {
        for (r, action) in hits {
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
        let fill = if selected {
            style
                .selected_background
                .map_or(self.theme.accent, |c| Color::new(c[0], c[1], c[2], c[3]))
        } else {
            style
                .background
                .map_or(self.theme.surface, |c| Color::new(c[0], c[1], c[2], c[3]))
        };

        let ink = style.text.map_or(
            if selected {
                self.theme.button_text
            } else {
                self.theme.text
            },
            |c| Color::new(c[0], c[1], c[2], c[3]),
        );

        let border = if selected {
            self.theme.accent
        } else {
            self.theme.border
        };

        let w = camera.screen_size.x;
        let h = camera.screen_size.y;

        self.rounded_rect(r, 7.0, border, w, h);

        self.rounded_rect(
            [r[0] + 1.0, r[1] + 1.0, r[2] - 1.0, r[3] - 1.0],
            6.0,
            fill,
            w,
            h,
        );

        let capacity = ((r[2] - r[0] - 14.0) / 7.4).max(1.0) as usize;

        let label = if label.chars().count() > capacity {
            let prefix: String = label.chars().take(capacity.saturating_sub(3)).collect();
            format!("{prefix}...")
        } else {
            label.to_owned()
        };

        self.text(
            camera,
            r[0] + 7.0,
            r[1] + ((r[3] - r[1] - 17.5) * 0.5).max(1.0),
            14.0,
            label,
            ink,
        );

        self.hits.push((r, action));
    }
}

fn shape_screen_bounds(shape: &ShapeInstance, w: f32, h: f32) -> [f32; 4] {
    let left = (shape.center[0] - shape.size[0] * 0.5 + 1.0) * w * 0.5;
    let top = (1.0 - shape.center[1] - shape.size[1] * 0.5) * h * 0.5;

    [
        left,
        top,
        left + shape.size[0] * w * 0.5,
        top + shape.size[1] * h * 0.5,
    ]
}

fn intersect(a: [f32; 4], clip: [f32; 4]) -> Option<[f32; 4]> {
    let r = [
        a[0].max(clip[0]),
        a[1].max(clip[1]),
        a[2].min(clip[2]),
        a[3].min(clip[3]),
    ];

    (r[2] > r[0] && r[3] > r[1]).then_some(r)
}

fn line_inside_clip(x: f32, y: f32, line_height: f32, clip: [f32; 4]) -> bool {
    x >= clip[0] && x < clip[2] && y >= clip[1] && y + line_height <= clip[3]
}
