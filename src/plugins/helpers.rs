use anyhow::{Result, bail, ensure};
use serde_json::{Value, json};
use unicode_segmentation::UnicodeSegmentation;

pub struct Helpers {
    fonts: glyphon::FontSystem,
}

impl Helpers {
    pub fn new() -> Self {
        let mut fonts = glyphon::FontSystem::new();
        fonts
            .db_mut()
            .load_font_data(include_bytes!("../../assets/Roboto-Black.ttf").to_vec());
        Self { fonts }
    }

    pub fn bridge(&mut self, operation: &str, input: &str) -> String {
        match self.call(operation, input) {
            Ok(value) => format!("{{\"ok\":true,\"value\":{value}}}"),
            Err(error) => json!({"ok":false,"error":format!("{error:#}")}).to_string(),
        }
    }

    fn call(&mut self, operation: &str, input: &str) -> Result<String> {
        ensure!(input.len() < 524288, "helper input limit");
        let args: Value = serde_json::from_str(input)?;
        let text = args["text"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("expected UTF-8 string"))?;
        let result = match operation {
            "graphemes" => {
                ensure!(text.len() <= 262144, "text limit");
                json!(text.graphemes(true).collect::<Vec<_>>())
            }
            "text_layout" => {
                let size = args["size"]
                    .as_f64()
                    .ok_or_else(|| anyhow::anyhow!("expected font size"))?;
                ensure!(
                    text.len() <= 32768 && size.is_finite() && (4.0..=512.0).contains(&size),
                    "text layout limit"
                );
                layout(&mut self.fonts, text, size as f32)
            }
            _ => bail!("unknown helper operation"),
        };
        let out = serde_json::to_string(&result)?;
        ensure!(out.len() < 1048576, "helper output limit");
        Ok(out)
    }
}

// HIGH COMPLEXITY TODO
fn layout(fonts: &mut glyphon::FontSystem, text: &str, size: f32) -> Value {
    use glyphon::{Attrs, Buffer, Family, Metrics, Shaping, Wrap};
    let mut buffer = Buffer::new(fonts, Metrics::new(size, size * 1.25));
    buffer.set_wrap(Wrap::None);
    buffer.set_text(
        text,
        &Attrs::new().family(Family::Name("Roboto")),
        Shaping::Advanced,
        None,
    );
    buffer.shape_until_scroll(fonts, false);
    let gs: Vec<_> = text.grapheme_indices(true).collect();
    let mut carets = vec![json!({"x":0.0,"y":0.0,"line":0}); gs.len() + 1];
    let mut lines = Vec::new();
    let mut byte_base = 0usize;
    let mut line_i = 0usize;
    let mut width = 0.0f32;
    for run in buffer.layout_runs() {
        while line_i < run.line_i {
            let rem = &text[byte_base..];
            byte_base += rem.find('\n').map_or(rem.len(), |n| n + 1);
            line_i += 1;
        }
        width = width.max(run.line_w);
        let start = gs.partition_point(|(b, _)| *b < byte_base);
        let end = gs.partition_point(|(b, _)| *b < byte_base + run.text.len());
        for idx in start..=end.min(gs.len()) {
            let byte = gs
                .get(idx)
                .map_or(text.len(), |(b, _)| *b)
                .saturating_sub(byte_base)
                .min(run.text.len());
            let x = if let Some(g) = run.glyphs.iter().find(|g| g.start <= byte && byte < g.end) {
                let count = run.text[g.start..g.end].graphemes(true).count().max(1) as f32;
                let before = run.text[g.start..byte].graphemes(true).count() as f32 / count;
                if g.level.is_rtl() {
                    g.x + g.w * (1.0 - before)
                } else {
                    g.x + g.w * before
                }
            } else if let Some(g) = run.glyphs.last() {
                if g.level.is_rtl() { g.x } else { g.x + g.w }
            } else {
                0.0
            };
            carets[idx] = json!({"x":x,"y":run.line_top,"line":run.line_i});
        }
        lines.push(json!({"start":start,"end":end,"y":run.line_top,"width":run.line_w}));
    }
    if lines.is_empty() {
        lines.push(json!({"start":0,"end":0,"y":0.0,"width":0.0}));
    }
    json!({"width":width,"height":lines.len() as f32*size*1.25,"line_height":size*1.25,"carets":carets,"lines":lines})
}
