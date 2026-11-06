use anyhow::{Result, bail, ensure};
use glyphon::{LayoutGlyph, LayoutRun};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use unicode_segmentation::UnicodeSegmentation;

const LINE_HEIGHT_FACTOR: f32 = 1.25;
const MAX_HELPER_INPUT_BYTES: usize = 524_288;
const MAX_HELPER_OUTPUT_BYTES: usize = 1_048_576;
const MAX_GRAPHEME_TEXT_BYTES: usize = 262_144;
const MAX_LAYOUT_TEXT_BYTES: usize = 32_768;
const MIN_LAYOUT_FONT_SIZE: f64 = 4.0;
const MAX_LAYOUT_FONT_SIZE: f64 = 512.0;
const MAX_ASSET_READ_BYTES: usize = 262_144;
const FONT_EXTENSIONS: &[&str] = &["ttf", "otf", "ttc"];

pub struct Helpers {
    fonts: glyphon::FontSystem,
    fonts_loaded: usize,
    assets: BTreeMap<String, Vec<u8>>,
}

impl Helpers {
    pub fn new(assets: BTreeMap<String, Vec<u8>>) -> Self {
        let mut fonts_loaded = 0;
        let fonts = crate::graphics::fonts::font_system(&mut fonts_loaded);
        Self {
            fonts,
            fonts_loaded,
            assets,
        }
    }

    pub fn bridge(&mut self, operation: &str, input: &str) -> String {
        match self.call(operation, input) {
            Ok(value) => format!("{{\"ok\":true,\"value\":{value}}}"),
            Err(error) => json!({"ok":false,"error":format!("{error:#}")}).to_string(),
        }
    }

    fn call(&mut self, operation: &str, input: &str) -> Result<String> {
        ensure!(input.len() < MAX_HELPER_INPUT_BYTES, "helper input limit");

        let args: Value = serde_json::from_str(input)?;

        let result = match operation {
            "graphemes" => {
                let text = text_arg(&args)?;
                ensure!(text.len() <= MAX_GRAPHEME_TEXT_BYTES, "text limit");
                json!(text.graphemes(true).collect::<Vec<_>>())
            }
            "text_layout" => {
                let text = text_arg(&args)?;
                let size = args["size"]
                    .as_f64()
                    .ok_or_else(|| anyhow::anyhow!("expected font size"))?;

                ensure!(
                    text.len() <= MAX_LAYOUT_TEXT_BYTES
                        && size.is_finite()
                        && (MIN_LAYOUT_FONT_SIZE..=MAX_LAYOUT_FONT_SIZE).contains(&size),
                    "text layout limit"
                );

                let family = crate::graphics::fonts::resolve(args["font"].as_str());
                crate::graphics::fonts::sync(&mut self.fonts, &mut self.fonts_loaded);
                layout(&mut self.fonts, text, size as f32, &family)
            }
            "asset_list" => json!(
                self.assets
                    .iter()
                    .map(|(path, bytes)| json!({"path": path, "size": bytes.len()}))
                    .collect::<Vec<_>>()
            ),
            "asset_read" => {
                let bytes = self.asset(&args)?;
                ensure!(
                    bytes.len() <= MAX_ASSET_READ_BYTES,
                    "asset too large to read"
                );
                json!(
                    std::str::from_utf8(bytes)
                        .map_err(|_| anyhow::anyhow!("asset is not UTF-8 text"))?
                )
            }
            "font_load" => {
                let path = args["path"].as_str().unwrap_or_default();
                ensure!(
                    std::path::Path::new(path)
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .is_some_and(
                            |ext| FONT_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str())
                        ),
                    "font must be a .ttf/.otf/.ttc asset"
                );
                let bytes = self.asset(&args)?.clone();
                json!(crate::graphics::fonts::register(bytes)?)
            }
            "font_list" => json!(crate::graphics::fonts::families()),
            _ => bail!("unknown helper operation"),
        };

        let out = serde_json::to_string(&result)?;
        ensure!(out.len() < MAX_HELPER_OUTPUT_BYTES, "helper output limit");
        Ok(out)
    }
}

fn text_arg(args: &Value) -> Result<&str> {
    args["text"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("expected UTF-8 string"))
}

impl Helpers {
    fn asset(&self, args: &Value) -> Result<&Vec<u8>> {
        let path = args["path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("expected asset path"))?;
        super::manifest::relative_name(path)?;
        ensure!(
            path.starts_with("assets/"),
            "only assets/ files are readable"
        );
        self.assets
            .get(path)
            .ok_or_else(|| anyhow::anyhow!("asset not found: {path}"))
    }
}

fn shaped_buffer(
    fonts: &mut glyphon::FontSystem,
    text: &str,
    size: f32,
    family: &str,
) -> glyphon::Buffer {
    use glyphon::{Attrs, Buffer, Family, Metrics, Shaping, Wrap};

    let mut buffer = Buffer::new(fonts, Metrics::new(size, size * LINE_HEIGHT_FACTOR));
    buffer.set_wrap(Wrap::None);
    buffer.set_text(
        text,
        &Attrs::new().family(Family::Name(family)),
        Shaping::Advanced,
        None,
    );
    buffer.shape_until_scroll(fonts, false);
    buffer
}

struct LineCursor<'a> {
    text: &'a str,
    byte_base: usize,
    line_i: usize,
}

impl LineCursor<'_> {
    fn seek(&mut self, target: usize) -> usize {
        let text = self.text;

        while self.line_i < target {
            let rest = &text[self.byte_base..];
            self.byte_base += rest.find('\n').map_or(rest.len(), |n| n + 1);
            self.line_i += 1;
        }

        self.byte_base
    }
}

fn fraction_before(run_text: &str, start: usize, end: usize, byte: usize) -> f32 {
    let count = run_text[start..end].graphemes(true).count().max(1) as f32;
    run_text[start..byte].graphemes(true).count() as f32 / count
}

fn layout(fonts: &mut glyphon::FontSystem, text: &str, size: f32, family: &str) -> Value {
    let buffer = shaped_buffer(fonts, text, size, family);
    let graphemes: Vec<_> = text.grapheme_indices(true).collect();

    let mut carets = vec![default_caret(); graphemes.len() + 1];
    let mut lines = Vec::new();
    let mut cursor = LineCursor {
        text,
        byte_base: 0,
        line_i: 0,
    };
    let mut width = 0.0f32;

    for run in buffer.layout_runs() {
        let byte_base = cursor.seek(run.line_i);
        width = width.max(run.line_w);

        let (start, end) = grapheme_range(&graphemes, byte_base, run.text.len());
        assign_carets(&mut carets, &graphemes, text, &run, byte_base, start, end);

        lines.push(line_json(start, end, &run));
    }

    if lines.is_empty() {
        lines.push(empty_line_json());
    }

    layout_json(width, size, &carets, &lines)
}

fn default_caret() -> Value {
    json!({"x": 0.0, "y": 0.0, "line": 0})
}

fn empty_line_json() -> Value {
    json!({"start": 0, "end": 0, "y": 0.0, "width": 0.0})
}

fn line_json(start: usize, end: usize, run: &LayoutRun) -> Value {
    json!({"start": start, "end": end, "y": run.line_top, "width": run.line_w})
}

fn layout_json(width: f32, size: f32, carets: &[Value], lines: &[Value]) -> Value {
    json!({
        "width": width,
        "height": lines.len() as f32 * size * LINE_HEIGHT_FACTOR,
        "line_height": size * LINE_HEIGHT_FACTOR,
        "carets": carets,
        "lines": lines,
    })
}

fn grapheme_range(graphemes: &[(usize, &str)], byte_base: usize, run_len: usize) -> (usize, usize) {
    let start = graphemes.partition_point(|(b, _)| *b < byte_base);
    let end = graphemes.partition_point(|(b, _)| *b < byte_base + run_len);
    (start, end)
}

fn assign_carets(
    carets: &mut [Value],
    graphemes: &[(usize, &str)],
    text: &str,
    run: &LayoutRun,
    byte_base: usize,
    start: usize,
    end: usize,
) {
    debug_assert_eq!(carets.len(), graphemes.len() + 1);
    let end = end.min(graphemes.len());
    let run_byte = |gb: usize| gb.saturating_sub(byte_base).min(run.text.len());
    let caret =
        |byte: usize| json!({"x": caret_x(run, byte), "y": run.line_top, "line": run.line_i});

    for (slot, &(gb, _)) in carets[start..end].iter_mut().zip(&graphemes[start..end]) {
        *slot = caret(run_byte(gb));
    }

    let tail = graphemes.get(end).map_or(text.len(), |(b, _)| *b);
    if let Some(slot) = carets.get_mut(end) {
        *slot = caret(run_byte(tail));
    }
}

fn caret_x(run: &LayoutRun, byte: usize) -> f32 {
    match run.glyphs.iter().find(|g| g.start <= byte && byte < g.end) {
        Some(g) => cluster_x(g, run.text, byte),
        None => run_end_x(run),
    }
}

fn cluster_x(g: &LayoutGlyph, run_text: &str, byte: usize) -> f32 {
    let before = fraction_before(run_text, g.start, g.end, byte);
    if g.level.is_rtl() {
        g.x + g.w * (1.0 - before)
    } else {
        g.x + g.w * before
    }
}

fn run_end_x(run: &LayoutRun) -> f32 {
    match run.glyphs.last() {
        Some(g) if g.level.is_rtl() => g.x,
        Some(g) => g.x + g.w,
        None => 0.0,
    }
}
