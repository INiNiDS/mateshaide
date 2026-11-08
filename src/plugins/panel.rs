use super::{PluginManager, UiAction, UiFrame, render::num};
use crate::{Camera, Color};
use anyhow::{Context, Result, bail, ensure};
use glam::Vec2;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::fmt::Write as _;

const MAX_PANEL_ITEMS: usize = 2048;
const MAX_PANEL_TEXT_BYTES: usize = 4096;
const MAX_BUTTON_ID_BYTES: usize = 128;
const MAX_HELP_PAGES_PER_PLUGIN: usize = 32;
const MAX_HELP_PAGE_BYTES: usize = 65_536;
const WHEEL_LINE_PX: f32 = 40.0;

pub struct Panel {
    rect: [f32; 4],
    items: Vec<Value>,
    scroll_max: f32,
}

#[derive(Default)]
pub struct PanelHost {
    panels: BTreeMap<String, Panel>,
    pages: Value,
    pages_key: String,
}

impl PluginManager {
    pub fn refresh_panels(&mut self, camera: &Camera) {
        let ids: Vec<String> = self
            .entries
            .iter()
            .filter(|(_, e)| e.runtime.as_ref().is_some_and(|r| r.has("panel")))
            .map(|(id, _)| id.clone())
            .collect();
        self.panel_host.panels.retain(|id, _| ids.contains(id));
        if ids.is_empty() {
            return;
        }
        self.refresh_help_pages();
        let screen = [camera.screen_size.x, camera.screen_size.y];
        let top = self.panel_top(camera);
        for id in ids {
            let reads_help = self.entries[&id].package.manifest.permits("help.read");
            let ctx = json!({
                "width": screen[0],
                "height": screen[1],
                "top": top,
                "dark": self.settings.appearance.dark,
                "pages": if reads_help { self.panel_host.pages.clone() } else { Value::Null },
            });
            let result = self
                .call_panel(&id, ctx.clone(), screen)
                .or_else(|_| self.call_panel(&id, ctx, screen));
            match result {
                Ok(Some(panel)) => {
                    self.panel_host.panels.insert(id, panel);
                }
                Ok(None) => {
                    self.panel_host.panels.remove(&id);
                }
                Err(e) => {
                    self.panel_host.panels.remove(&id);
                    self.notice = format!("Panel {id}: {e:#}");
                }
            }
        }
    }

    fn call_panel(&mut self, id: &str, ctx: Value, screen: [f32; 2]) -> Result<Option<Panel>> {
        let runtime = self
            .entries
            .get_mut(id)
            .and_then(|e| e.runtime.as_mut())
            .context("plugin is not active")?;
        let out = runtime.cached_pure_call("panel", "panel", &[ctx], true, || Ok(()))?;
        parse_panel(&out, screen)
    }

    fn refresh_help_pages(&mut self) {
        let mut key = String::new();
        for (id, e) in &self.entries {
            let _ = write!(
                key,
                "{id}:{}:{};",
                e.package.fingerprint,
                e.runtime.is_some()
            );
        }
        if key == self.panel_host.pages_key {
            return;
        }
        let mut pages = Vec::new();
        for (id, e) in &self.entries {
            let files = e
                .package
                .files
                .iter()
                .filter_map(|(path, bytes)| {
                    let file = path.strip_prefix("assets/help/")?;
                    let ok = std::path::Path::new(file)
                        .extension()
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
                        && !file.contains('/')
                        && bytes.len() <= MAX_HELP_PAGE_BYTES;
                    let text = std::str::from_utf8(bytes).ok().filter(|_| ok)?;
                    Some((file, text))
                })
                .take(MAX_HELP_PAGES_PER_PLUGIN);
            for (file, text) in files {
                pages.push(json!({
                    "plugin": id,
                    "plugin_name": e.package.manifest.name,
                    "active": e.runtime.is_some(),
                    "file": file,
                    "text": text.replace('\r', ""),
                }));
            }
        }
        self.panel_host.pages = Value::Array(pages);
        self.panel_host.pages_key = key;
    }

    pub(super) fn draw_panels(&self, frame: &mut UiFrame, camera: &Camera) {
        for (id, panel) in &self.panel_host.panels {
            let r = panel.rect;
            frame.floating_panel(camera, r);
            let mut body = UiFrame::with_theme(frame.theme);
            for item in &panel.items {
                let _ = draw_item(&mut body, camera, id, [r[0], r[1]], item);
            }
            frame.append_clipped(
                body,
                camera,
                [r[0] + 1.0, r[1] + 1.0, r[2] - 1.0, r[3] - 1.0],
            );
        }
    }

    #[must_use]
    pub fn panel_at(&self, p: Vec2) -> Option<(String, f32)> {
        self.panel_host.panels.iter().rev().find_map(|(id, panel)| {
            let r = panel.rect;
            (p.x >= r[0] && p.y >= r[1] && p.x <= r[2] && p.y <= r[3])
                .then(|| (id.clone(), panel.scroll_max))
        })
    }
}

fn parse_panel(out: &Value, [w, h]: [f32; 2]) -> Result<Option<Panel>> {
    if !out.is_object() || out.as_object().is_some_and(serde_json::Map::is_empty) {
        return Ok(None);
    }
    let (x, y) = (num(out, "x")?, num(out, "y")?);
    let (width, height) = (num(out, "width")?, num(out, "height")?);
    ensure!(width >= 0.0 && height >= 0.0, "negative panel size");
    let items = out["items"].as_array().cloned().unwrap_or_default();
    ensure!(items.len() <= MAX_PANEL_ITEMS, "panel item limit");
    Ok(Some(Panel {
        rect: [
            x.clamp(0.0, w),
            y.clamp(0.0, h),
            (x + width).clamp(0.0, w),
            (y + height).clamp(0.0, h),
        ],
        items,
        scroll_max: num(out, "scroll_max").unwrap_or(0.0).max(0.0),
    }))
}

fn draw_item(
    body: &mut UiFrame,
    camera: &Camera,
    plugin: &str,
    [ox, oy]: [f32; 2],
    item: &Value,
) -> Result<()> {
    let (x, y) = (ox + num(item, "x")?, oy + num(item, "y")?);
    let (w, h) = (camera.screen_size.x, camera.screen_size.y);
    match item["kind"].as_str().unwrap_or_default() {
        "text" => {
            let text = item["text"].as_str().context("missing text")?;
            ensure!(text.len() <= MAX_PANEL_TEXT_BYTES, "panel text limit");
            let size = num(item, "size").unwrap_or(13.0).clamp(8.0, 48.0);
            let color = theme_color(body, &item["color"]);
            body.text(camera, x, y, size, text, color);
        }
        "rect" => {
            let r = [x, y, x + num(item, "width")?, y + num(item, "height")?];
            let color = theme_color(body, &item["color"]);
            body.rect(r, color, w, h);
        }
        "button" => {
            let id = item["id"].as_str().context("missing button id")?;
            ensure!(
                !id.is_empty() && id.len() <= MAX_BUTTON_ID_BYTES,
                "invalid button id"
            );
            let r = [x, y, x + num(item, "width")?, y + num(item, "height")?];
            let label = item["label"].as_str().unwrap_or_default();
            let action = UiAction::PanelButton(plugin.to_owned(), id.to_owned());
            body.button(camera, r, label, action, item["selected"] == true);
        }
        _ => bail!("unknown panel item"),
    }
    Ok(())
}

fn theme_color(frame: &UiFrame, name: &Value) -> Color {
    let t = &frame.theme;
    match name.as_str() {
        Some("muted") => t.muted,
        Some("accent") => t.accent,
        Some("warning") => t.warning,
        Some("error") => t.error,
        Some("success") => t.success,
        Some("border") => t.border,
        Some("surface") => t.surface,
        _ => t.text,
    }
}

impl crate::Engine {
    pub(crate) fn handle_panel_wheel(
        &mut self,
        event: &winit::event::WindowEvent,
        position: Vec2,
    ) -> bool {
        let winit::event::WindowEvent::MouseWheel { delta, .. } = event else {
            return false;
        };
        if self.plugin_page.open {
            return false;
        }
        let Some((id, max)) = self.plugins.panel_at(position) else {
            return false;
        };
        let dy = match delta {
            winit::event::MouseScrollDelta::LineDelta(_, y) => -y * WHEEL_LINE_PX,
            winit::event::MouseScrollDelta::PixelDelta(p) => -(p.y as f32),
        };
        let event = json!({"name": "panel.scroll", "dy": dy, "max": max});
        self.plugins.dispatch(&id, &event, &mut self.canvas);
        true
    }
}
