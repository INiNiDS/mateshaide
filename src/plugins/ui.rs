use super::manager::Confirmation;
use super::{PluginManager, UiFrame};
use crate::{Camera, Color};
use std::fmt::Write;

const UI_MARGIN: f32 = 16.0;
const BUTTON_HEIGHT: f32 = 32.0;
const BUTTON_GAP: f32 = 6.0;
const PANEL_PAD: f32 = 9.0;
const PANEL_GAP: f32 = 10.0;

const PAGE_TOP: f32 = 80.0;
const PAGE_BOTTOM: f32 = 76.0;
const CARD_HEIGHT: f32 = 232.0;
const CARD_GAP: f32 = 20.0;

type Rect = [f32; 4];

#[derive(Clone)]
pub enum UiAction {
    TogglePage,
    SelectPlugin(String),
    TogglePlugin(String),
    Confirm,
    Cancel,
    Rescan,
    Repository(String),
    Tool(String, String),
    Command(String, String),
    Save,
    Open,
    Import,
    ToggleDark,
    ToggleHalloween,
}

#[derive(Default)]
pub struct PluginPage {
    pub open: bool,
    pub selected: Option<String>,
    pub scroll: f32,
    pub dialog_scroll: f32,
    pub quick_scroll: f32,
}

#[derive(Debug)]
struct TopLayout {
    system: Rect,
    system_buttons: [Rect; 3],
    brand: bool,

    themes: Rect,
    theme_buttons: [Rect; 2],

    tools: Option<Rect>,
    tool_buttons: Vec<Rect>,

    bottom: f32,
}

fn inset_buttons(panel: Rect, widths: &[f32], lead: f32) -> Vec<Rect> {
    let mut x = panel[0] + PANEL_PAD + lead;
    let y = panel[1] + PANEL_PAD;

    widths
        .iter()
        .map(|width| {
            let r = [x, y, x + width, y + BUTTON_HEIGHT];
            x += width + BUTTON_GAP;
            r
        })
        .collect()
}

// HIGH COMPLEXITY TODO
fn top_layout(width: f32, tool_widths: &[f32]) -> TopLayout {
    let available = (width - 2.0 * UI_MARGIN).max(1.0);
    let brand = width >= 700.0;
    let lead = if brand { 106.0 } else { 0.0 };

    let system_widths = [70.0_f32, 54.0, 54.0]
        .map(|v| v.min(((available - 2.0 * PANEL_PAD - 2.0 * BUTTON_GAP - lead) / 3.0).max(1.0)));

    let sw = system_widths.iter().sum::<f32>() + 2.0 * BUTTON_GAP + 2.0 * PANEL_PAD + lead;

    let system = [UI_MARGIN, UI_MARGIN, UI_MARGIN + sw, UI_MARGIN + 50.0];

    let theme_widths = [88.0_f32, 132.0]
        .map(|v| v.min(((available - 2.0 * PANEL_PAD - BUTTON_GAP) / 2.0).max(1.0)));

    let tw = theme_widths.iter().sum::<f32>() + BUTTON_GAP + 2.0 * PANEL_PAD;

    let ty = if sw + tw + PANEL_GAP <= available {
        UI_MARGIN
    } else {
        system[3] + PANEL_GAP
    };

    let themes = [width - UI_MARGIN - tw, ty, width - UI_MARGIN, ty + 50.0];

    let mut bottom = system[3].max(themes[3]);
    let mut tool_buttons = Vec::new();

    let tools = if tool_widths.is_empty() {
        None
    } else {
        let natural = tool_widths.iter().sum::<f32>()
            + tool_widths.len().saturating_sub(1) as f32 * BUTTON_GAP
            + 2.0 * PANEL_PAD;

        let panel_width = natural.min(available);
        let centered_left = (width - panel_width) * 0.5;

        let fits_top = ty == UI_MARGIN
            && centered_left >= system[2] + PANEL_GAP
            && centered_left + panel_width <= themes[0] - PANEL_GAP;

        let y = if fits_top {
            UI_MARGIN
        } else {
            bottom + PANEL_GAP
        };

        let left = centered_left;
        let mut x = left + PANEL_PAD;
        let mut by = y + PANEL_PAD;

        for &button_width in tool_widths {
            let bw = button_width.min((panel_width - 2.0 * PANEL_PAD).max(1.0));

            if x > left + PANEL_PAD && x + bw > left + panel_width - PANEL_PAD + 0.01 {
                x = left + PANEL_PAD;
                by += BUTTON_HEIGHT + BUTTON_GAP;
            }

            tool_buttons.push([x, by, x + bw, by + BUTTON_HEIGHT]);

            x += bw + BUTTON_GAP;
        }

        let panel = [left, y, left + panel_width, by + BUTTON_HEIGHT + PANEL_PAD];

        bottom = bottom.max(panel[3]);

        Some(panel)
    };

    let sb = inset_buttons(system, &system_widths, lead);
    let tb = inset_buttons(themes, &theme_widths, 0.0);

    TopLayout {
        system,
        system_buttons: [sb[0], sb[1], sb[2]],
        brand,
        themes,
        theme_buttons: [tb[0], tb[1]],
        tools,
        tool_buttons,
        bottom,
    }
}

struct CommandGroupLayout {
    plugin_id: String,
    bounds: Rect,
    buttons: Vec<Rect>,
}

struct QuickLayout {
    groups: Vec<CommandGroupLayout>,
    viewport: Rect,
    max_scroll: f32,
}

fn command_group(widths: &[f32], max_width: f32) -> (f32, f32, Vec<Rect>) {
    let natural = widths.iter().sum::<f32>()
        + widths.len().saturating_sub(1) as f32 * BUTTON_GAP
        + 2.0 * PANEL_PAD;

    let width = natural.max(150.0).min(max_width);
    let mut x = PANEL_PAD;
    let mut y = 27.0;
    let mut buttons = Vec::new();

    for &button_width in widths {
        let bw = button_width.min((width - 2.0 * PANEL_PAD).max(1.0));

        if x > PANEL_PAD && x + bw > width - PANEL_PAD + 0.01 {
            x = PANEL_PAD;
            y += BUTTON_HEIGHT + BUTTON_GAP;
        }

        buttons.push([x, y, x + bw, y + BUTTON_HEIGHT]);

        x += bw + BUTTON_GAP;
    }

    (width, y + BUTTON_HEIGHT + PANEL_PAD, buttons)
}

fn dialog_rect(w: f32, h: f32) -> [f32; 4] {
    let width = (w - 32.0).clamp(1.0, 520.0);
    let height = (h - 32.0).clamp(1.0, 380.0);

    let left = (w - width) * 0.5;
    let top = (h - height) * 0.5;

    [left, top, left + width, top + height]
}

fn clipped(s: &str, n: usize) -> String {
    let mut out: String = s.chars().take(n).collect();

    if s.chars().count() > n {
        out.push_str("...");
    }

    out
}

fn wrapped(s: &str, cols: usize) -> String {
    let mut out = String::new();
    let mut column = 0;

    for ch in s.chars() {
        if ch == '\n' {
            out.push(ch);
            column = 0;
            continue;
        }

        if column >= cols {
            out.push('\n');
            column = 0;
        }

        out.push(ch);
        column += 1;
    }

    out
}

fn text_width(label: &str) -> f32 {
    (label.chars().count() as f32 * 7.4 + 20.0).clamp(52.0, 200.0)
}

fn notice_columns(width: f32) -> usize {
    ((width - 24.0) / 7.0).max(20.0) as usize
}

impl PluginManager {
    #[must_use]
    pub fn ui_frame(&self, page: &PluginPage, camera: &Camera) -> UiFrame {
        let mut frame = UiFrame::with_theme(self.settings.appearance.theme());

        let screen = [camera.screen_size.x, camera.screen_size.y];

        if !page.open {
            self.draw_canvas_grid(&mut frame, camera);

            self.draw_halloween_decoration(&mut frame, camera, screen, false);

            self.draw_toolbar(&mut frame, page, camera, screen);

            self.draw_quick_actions(&mut frame, page, camera, screen);

            self.draw_notice(&mut frame, camera, screen);

            return frame;
        }

        self.draw_plugin_page(&mut frame, page, camera, screen);

        if self.confirmation.is_some() {
            self.draw_confirmation_dialog(&mut frame, page, camera, screen);
        }

        frame
    }

    fn toolbar_layout(&self, width: f32) -> TopLayout {
        let widths: Vec<_> = self
            .entries
            .values()
            .filter(|entry| entry.runtime.is_some())
            .flat_map(|entry| entry.contributions.tools.iter())
            .map(|tool| text_width(&tool.label))
            .collect();

        top_layout(width, &widths)
    }

    fn draw_canvas_grid(&self, frame: &mut UiFrame, camera: &Camera) {
        let w = camera.screen_size.x;
        let h = camera.screen_size.y;
        let mut step = 24.0 * camera.zoom;

        while step < 24.0 {
            step *= 2.0;
        }

        while step > 48.0 {
            step *= 0.5;
        }

        while (w / step + 2.0) * (h / step + 2.0) > 16384.0 {
            step *= 2.0;
        }

        let origin = camera.world_to_screen(glam::Vec2::ZERO);

        let ink = frame
            .theme
            .muted
            .with_alpha(if frame.theme.dark { 0.16 } else { 0.21 });

        let mut y = origin.y.rem_euclid(step);

        while y < h {
            let mut x = origin.x.rem_euclid(step);

            while x < w {
                frame.rect([x, y, x + 1.25, y + 1.25], ink, w, h);

                x += step;
            }

            y += step;
        }

        frame.background_shape_count = frame.shapes.len();
    }

    fn draw_toolbar(
        &self,
        frame: &mut UiFrame,
        _page: &PluginPage,
        camera: &Camera,
        [w, _h]: [f32; 2],
    ) {
        let appearance = self.settings.appearance;
        let layout = self.toolbar_layout(w);

        frame.floating_panel(camera, layout.system);

        if layout.brand {
            frame.text(
                camera,
                layout.system[0] + PANEL_PAD,
                layout.system[1] + 16.0,
                14.0,
                "MateshaIDE",
                frame.theme.text,
            );
        }

        for ((label, action), bounds) in [
            ("Plugins", UiAction::TogglePage),
            ("Open", UiAction::Open),
            ("Save", UiAction::Save),
        ]
        .into_iter()
        .zip(layout.system_buttons)
        {
            frame.button(camera, bounds, label, action, false);
        }

        frame.floating_panel(camera, layout.themes);

        frame.button(
            camera,
            layout.theme_buttons[0],
            if appearance.dark {
                "Dark: on"
            } else {
                "Dark: off"
            },
            UiAction::ToggleDark,
            appearance.dark,
        );

        frame.button(
            camera,
            layout.theme_buttons[1],
            if appearance.halloween {
                "Halloween: on"
            } else {
                "Halloween: off"
            },
            UiAction::ToggleHalloween,
            appearance.halloween,
        );

        if let Some(bounds) = layout.tools {
            frame.floating_panel(camera, bounds);
        }

        let mut buttons = layout.tool_buttons.into_iter();

        for (plugin_id, entry) in &self.entries {
            if entry.runtime.is_none() {
                continue;
            }

            for tool in &entry.contributions.tools {
                let Some(bounds) = buttons.next() else {
                    break;
                };

                let selected =
                    self.selected.as_ref() == Some(&(plugin_id.clone(), tool.id.clone()));

                frame.button_styled(
                    camera,
                    bounds,
                    &tool.label,
                    UiAction::Tool(plugin_id.clone(), tool.id.clone()),
                    selected,
                    &tool.style,
                );
            }
        }
    }

    fn draw_halloween_decoration(
        &self,
        frame: &mut UiFrame,
        _camera: &Camera,
        [w, h]: [f32; 2],
        plugin_page: bool,
    ) {
        use super::UiImageKind::{Blood, CornerWeb, Eyes, Mascot, Pumpkin, Spider, Strips, Web};

        if !frame.theme.halloween || self.confirmation.is_some() || w < 640.0 || h < 420.0 {
            return;
        }

        let scale = (w / 1280.0).min(h / 800.0).clamp(0.65, 1.25);

        frame.image(
            Blood,
            [-18.0 * scale, -28.0 * scale, 132.0 * scale, 122.0 * scale],
            [0.0, 0.0],
            false,
        );

        frame.image(
            CornerWeb,
            [w - 224.0 * scale, 0.0, w, 224.0 * scale],
            [1.0, 0.0],
            false,
        );

        let edge_top = if plugin_page {
            100.0
        } else {
            self.toolbar_layout(w).bottom + 98.0
        };

        if edge_top + 224.0 * scale < h - 132.0 * scale {
            frame.image(
                Spider,
                [w - 112.0 * scale, edge_top, w, edge_top + 224.0 * scale],
                [1.0, 0.0],
                false,
            );
        }

        if plugin_page {
            frame.image(
                Strips,
                [0.0, h * 0.45, 92.0 * scale, h * 0.45 + 92.0 * scale],
                [0.0, 0.0],
                false,
            );

            frame.image(
                Web,
                [w - 116.0 * scale, h - 116.0 * scale, w, h],
                [1.0, 1.0],
                true,
            );
        } else {
            let m = [
                14.0,
                h - 126.0 * scale,
                14.0 + 96.0 * scale,
                h - 30.0 * scale,
            ];

            frame.rounded_rect(m, 9.0, Color::hex(0xe4d8bf), w, h);

            frame.image(
                Mascot,
                [m[0] + 4.0, m[1] + 4.0, m[2] - 4.0, m[3] - 4.0],
                [0.5, 0.5],
                false,
            );

            if w >= 900.0 {
                frame.image(
                    Eyes,
                    [
                        w - 302.0 * scale,
                        h - 102.0 * scale,
                        w - 126.0 * scale,
                        h - 14.0 * scale,
                    ],
                    [1.0, 1.0],
                    false,
                );
            }
        }

        frame.image(
            Pumpkin,
            [w - 108.0 * scale, h - 108.0 * scale, w - 8.0, h - 8.0],
            [1.0, 1.0],
            false,
        );
    }

    fn quick_layout(&self, camera: &Camera) -> QuickLayout {
        let w = camera.screen_size.x;
        let h = camera.screen_size.y;

        let top = self.toolbar_layout(w).bottom + PANEL_GAP;
        let available = (w - 2.0 * UI_MARGIN).max(1.0);

        let mut x = UI_MARGIN;
        let mut y = top;
        let mut row_height = 0.0_f32;
        let mut groups = Vec::new();

        for (plugin_id, entry) in &self.entries {
            if entry.runtime.is_none() || entry.contributions.commands.is_empty() {
                continue;
            }

            let widths: Vec<_> = entry
                .contributions
                .commands
                .iter()
                .map(|command| text_width(&command.label))
                .collect();

            let (gw, gh, local) = command_group(&widths, available);

            if x > UI_MARGIN && x + gw > w - UI_MARGIN + 0.01 {
                x = UI_MARGIN;
                y += row_height + PANEL_GAP;
                row_height = 0.0;
            }

            let bounds = [x, y, x + gw, y + gh];

            let buttons = local
                .into_iter()
                .map(|b| [b[0] + x, b[1] + y, b[2] + x, b[3] + y])
                .collect();

            groups.push(CommandGroupLayout {
                plugin_id: plugin_id.clone(),
                bounds,
                buttons,
            });

            row_height = row_height.max(gh);
            x += gw + PANEL_GAP;
        }

        let content_height = if groups.is_empty() {
            0.0
        } else {
            y + row_height - top
        };

        let available_height = (h - top - 56.0).max(0.0).min((h * 0.42).max(68.0));

        let viewport_height = content_height.min(available_height);

        QuickLayout {
            groups,
            viewport: [UI_MARGIN, top, w - UI_MARGIN, top + viewport_height],
            max_scroll: (content_height - viewport_height).max(0.0),
        }
    }

    pub(crate) fn quick_scroll_max(&self, camera: &Camera) -> f32 {
        self.quick_layout(camera).max_scroll
    }

    pub(crate) fn quick_contains(&self, page: &PluginPage, camera: &Camera, p: glam::Vec2) -> bool {
        let layout = self.quick_layout(camera);
        let scroll = page.quick_scroll.clamp(0.0, layout.max_scroll);

        let v = layout.viewport;

        if p.x < v[0] || p.x > v[2] || p.y < v[1] || p.y > v[3] {
            return false;
        }

        layout.groups.iter().any(|group| {
            p.x >= group.bounds[0]
                && p.x <= group.bounds[2]
                && p.y >= group.bounds[1] - scroll
                && p.y <= group.bounds[3] - scroll
        })
    }

    fn draw_quick_actions(
        &self,
        frame: &mut UiFrame,
        page: &PluginPage,
        camera: &Camera,
        _screen: [f32; 2],
    ) {
        let layout = self.quick_layout(camera);

        let scroll = page.quick_scroll.clamp(0.0, layout.max_scroll);

        let mut body = UiFrame::with_theme(frame.theme);

        for group in &layout.groups {
            let Some(entry) = self.entries.get(&group.plugin_id) else {
                continue;
            };

            let shift = |r: Rect| [r[0], r[1] - scroll, r[2], r[3] - scroll];

            let bounds = shift(group.bounds);

            if bounds[3] <= layout.viewport[1] || bounds[1] >= layout.viewport[3] {
                continue;
            }

            body.floating_panel(camera, bounds);

            let title = entry
                .contributions
                .tools
                .first()
                .map_or(entry.package.manifest.name.as_str(), |tool| {
                    tool.label.as_str()
                });

            body.text(
                camera,
                bounds[0] + PANEL_PAD,
                bounds[1] + 7.0,
                11.0,
                clipped(
                    title,
                    ((bounds[2] - bounds[0] - 20.0) / 7.0).max(1.0) as usize,
                ),
                frame.theme.muted,
            );

            for (command, button) in entry.contributions.commands.iter().zip(&group.buttons) {
                body.button_styled(
                    camera,
                    shift(*button),
                    &command.label,
                    UiAction::Command(group.plugin_id.clone(), command.id.clone()),
                    false,
                    &command.style,
                );
            }
        }

        frame.append_clipped(body, camera, layout.viewport);

        if layout.max_scroll > 0.0 && layout.viewport[3] > layout.viewport[1] {
            let v = layout.viewport;
            let height = v[3] - v[1];

            let thumb = (height * height / (height + layout.max_scroll))
                .max(12.0)
                .min(height);

            let y = v[1] + (height - thumb) * scroll / layout.max_scroll;

            frame.rect(
                [v[2] - 3.0, y, v[2], y + thumb],
                frame.theme.muted.with_alpha(0.7),
                camera.screen_size.x,
                camera.screen_size.y,
            );
        }
    }

    fn draw_notice(&self, frame: &mut UiFrame, camera: &Camera, [w, h]: [f32; 2]) {
        if self.notice.is_empty() {
            return;
        }

        let width = (w - 2.0 * UI_MARGIN).clamp(1.0, 620.0);

        let left = (w - width) * 0.5;

        let r = [left, h - 48.0, left + width, h - 14.0];

        frame.floating_panel(camera, r);

        frame.text(
            camera,
            left + 10.0,
            h - 39.0,
            12.0,
            clipped(&self.notice, ((width - 20.0) / 7.0).max(1.0) as usize),
            frame.theme.warning,
        );
    }

    // HIGH COMPLEXITY TODO
    fn draw_plugin_page(
        &self,
        frame: &mut UiFrame,
        page: &PluginPage,
        camera: &Camera,
        [w, h]: [f32; 2],
    ) {
        let theme = self.settings.appearance.theme();

        frame.rect([0.0, 0.0, w, h], theme.panel, w, h);

        self.draw_halloween_decoration(frame, camera, [w, h], true);

        let [left, right] = self.plugin_card_bounds(w, h);
        let width = right - left;
        let bottom = (h - PAGE_BOTTOM).max(PAGE_TOP);

        let scroll = page.scroll.clamp(0.0, self.ui_scroll_max(page, camera));

        let mut body = UiFrame::with_theme(frame.theme);

        for (index, (plugin_id, entry)) in self.entries.iter().enumerate() {
            let y = PAGE_TOP + index as f32 * (CARD_HEIGHT + CARD_GAP) - scroll;

            if y + CARD_HEIGHT <= PAGE_TOP || y >= bottom {
                continue;
            }

            let manifest = &entry.package.manifest;
            let active = entry.runtime.is_some();

            let icon_size = if width >= 600.0 { 96.0 } else { 64.0 };

            let x = left + icon_size + 44.0;

            let columns = ((right - x - 20.0) / 8.0).max(1.0) as usize;

            body.rect([left, y, right, y + CARD_HEIGHT], theme.border, w, h);

            body.rect(
                [left + 1.0, y + 1.0, right - 1.0, y + CARD_HEIGHT - 1.0],
                theme.surface,
                w,
                h,
            );

            body.rect(
                [
                    left + 20.0,
                    y + 24.0,
                    left + 20.0 + icon_size,
                    y + 24.0 + icon_size,
                ],
                theme.surface,
                w,
                h,
            );

            if let Some(icon) = &entry.package.icon {
                let scale = icon_size / icon.width().max(icon.height()).max(1) as f32;

                for (row, pixels) in icon.rows().enumerate() {
                    for (column, pixel) in pixels.enumerate() {
                        if pixel[3] == 0 {
                            continue;
                        }

                        let px = left + 20.0 + column as f32 * scale;

                        let py = y + 24.0 + row as f32 * scale;

                        body.rect(
                            [px, py, px + scale, py + scale],
                            Color::rgba8(pixel[0], pixel[1], pixel[2], pixel[3]),
                            w,
                            h,
                        );
                    }
                }
            }

            let title_columns = ((right - x - 20.0) / 12.0).max(1.0) as usize;

            body.text(
                camera,
                x,
                y + 18.0,
                21.0,
                clipped(&manifest.name, title_columns),
                theme.text,
            );

            body.text(
                camera,
                x,
                y + 48.0,
                12.0,
                clipped(&format!("{}  v{}", manifest.id, manifest.version), columns),
                theme.muted,
            );

            let description = wrapped(&manifest.description, columns)
                .lines()
                .take(3)
                .collect::<Vec<_>>()
                .join("\n");

            body.text(camera, x, y + 74.0, 14.0, description, theme.text);

            body.text(
                camera,
                x,
                y + 136.0,
                12.0,
                clipped(
                    &format!(
                        "Permissions: {} | Dependencies: {}",
                        manifest.permissions.len(),
                        manifest.dependencies.len(),
                    ),
                    columns,
                ),
                theme.muted,
            );

            let error = if active {
                None
            } else {
                self.plan(plugin_id).err()
            };

            let (status, status_color) = if let Some(error) = error {
                (
                    clipped(&format!("Blocked: {error:#}"), columns),
                    theme.error,
                )
            } else if active {
                ("Enabled".to_owned(), theme.success)
            } else {
                ("Disabled".to_owned(), theme.muted)
            };

            body.text(camera, x, y + 159.0, 12.0, status, status_color);

            let button_width = ((right - x - 28.0) * 0.5).clamp(1.0, 126.0);

            body.button(
                camera,
                [x, y + 188.0, x + button_width, y + 220.0],
                if active { "Disable" } else { "Activate" },
                UiAction::TogglePlugin(plugin_id.clone()),
                active,
            );

            body.button(
                camera,
                [
                    x + button_width + 8.0,
                    y + 188.0,
                    x + button_width * 2.0 + 8.0,
                    y + 220.0,
                ],
                "Repository",
                UiAction::Repository(manifest.repository.clone()),
                false,
            );
        }

        frame.append_clipped(body, camera, [left, PAGE_TOP, right, bottom]);

        if self.entries.is_empty() {
            frame.text(
                camera,
                left + 20.0,
                PAGE_TOP + 24.0,
                18.0,
                "No plugins installed.",
                theme.muted,
            );
        }

        self.draw_page_header(frame, page, camera, w);
        self.draw_page_footer(frame, camera, w, h);
    }

    fn draw_page_header(&self, frame: &mut UiFrame, _page: &PluginPage, camera: &Camera, w: f32) {
        let theme = self.settings.appearance.theme();

        frame.text(camera, 22.0, 18.0, 24.0, "Plugins", theme.text);

        frame.button(
            camera,
            [w - 108.0, 14.0, w - 16.0, 46.0],
            "Close",
            UiAction::TogglePage,
            false,
        );

        frame.button(
            camera,
            [150.0, 14.0, 252.0, 46.0],
            "Rescan F5",
            UiAction::Rescan,
            false,
        );

        if w > 540.0 {
            frame.button(
                camera,
                [262.0, 14.0, 382.0, 46.0],
                "Import ZIP",
                UiAction::Import,
                false,
            );
        }
    }

    fn plugin_card_bounds(&self, w: f32, h: f32) -> [f32; 2] {
        let scale = (w / 1000.0).min(h / 700.0).clamp(0.65, 1.4);

        let gutter = if self.settings.appearance.halloween && w >= 640.0 {
            (160.0 * scale).min(w * 0.24) + 24.0 * scale
        } else {
            20.0
        };

        let width = (w - 2.0 * gutter).clamp(1.0, 820.0);
        let left = (w - width) * 0.5;

        [left, left + width]
    }

    fn draw_page_footer(&self, frame: &mut UiFrame, camera: &Camera, w: f32, h: f32) {
        let theme = self.settings.appearance.theme();

        frame.text(
            camera,
            14.0,
            h - 58.0,
            12.0,
            clipped(&self.notice, notice_columns(w)),
            theme.warning,
        );

        frame.text(
            camera,
            14.0,
            h - 30.0,
            12.0,
            "Install: clone/unpack into plugins/, then Rescan. Browser: use Import ZIP in its build.",
            theme.muted,
        );
    }

    fn draw_confirmation_dialog(
        &self,
        frame: &mut UiFrame,
        page: &PluginPage,
        camera: &Camera,
        [w, h]: [f32; 2],
    ) {
        let Some(confirm) = &self.confirmation else {
            return;
        };

        let theme = self.settings.appearance.theme();

        frame.texts.clear();
        frame.hits.clear();

        frame.rect([0.0, 0.0, w, h], Color::new(0.0, 0.0, 0.0, 0.24), w, h);

        let [left, top, right, bottom] = dialog_rect(w, h);

        frame.rect(
            [left + 5.0, top + 5.0, right + 5.0, bottom + 5.0],
            Color::new(0.0, 0.0, 0.0, 0.12),
            w,
            h,
        );

        frame.rect(
            [left - 1.0, top - 1.0, right + 1.0, bottom + 1.0],
            theme.border,
            w,
            h,
        );

        frame.rect([left, top, right, bottom], theme.panel, w, h);

        let (title, label, count) = match confirm {
            Confirmation::Enable(ids) => ("Activate plugins?", "Activate", ids.len()),
            Confirmation::Disable(ids) => ("Disable plugins?", "Disable", ids.len()),
        };

        frame.text(camera, left + 20.0, top + 16.0, 20.0, title, theme.text);

        frame.text(
            camera,
            left + 20.0,
            top + 43.0,
            12.0,
            format!("{count} plugin(s). Scroll inside to review."),
            theme.muted,
        );

        let columns = ((right - left - 40.0) / 8.0).max(1.0) as usize;

        let scroll = page
            .dialog_scroll
            .clamp(0.0, self.ui_scroll_max(page, camera));

        let mut body = UiFrame::default();

        body.text(
            camera,
            left + 20.0,
            top + 64.0 - scroll,
            14.0,
            self.confirmation_body(confirm, columns),
            theme.text,
        );

        frame.append_clipped(
            body,
            camera,
            [left + 20.0, top + 64.0, right - 20.0, bottom - 68.0],
        );

        frame.button(
            camera,
            [right - 228.0, bottom - 48.0, right - 120.0, bottom - 16.0],
            label,
            UiAction::Confirm,
            false,
        );

        frame.button(
            camera,
            [right - 108.0, bottom - 48.0, right - 16.0, bottom - 16.0],
            "Cancel",
            UiAction::Cancel,
            false,
        );
    }

    fn confirmation_warning<'a>(&self, confirm: &'a Confirmation) -> &'a str {
        if matches!(confirm, Confirmation::Enable(_)) {
            "Plugin code runs after confirmation. Requested permissions allow changes to your document or clipboard. Sandboxing is not a guarantee against all bugs or resource exhaustion. Activate only code you trust. Dependencies listed below will also be activated."
        } else {
            "Dependent plugins must stop too. Created objects remain in the document and will show placeholders until their plugins are enabled again."
        }
    }

    fn confirmation_body(&self, confirm: &Confirmation, columns: usize) -> String {
        let ids = match confirm {
            Confirmation::Enable(ids) | Confirmation::Disable(ids) => ids,
        };

        let mut text = self.confirmation_warning(confirm).to_owned();

        for id in ids {
            let Some(entry) = self.entries.get(id) else {
                continue;
            };

            let manifest = &entry.package.manifest;

            let permissions = if manifest.permissions.is_empty() {
                "none".to_owned()
            } else {
                manifest.permissions.join(", ")
            };

            let _ = write!(
                text,
                "\n\n{} v{}\n{}\nAuthors: {}\nPermissions: {}",
                manifest.name,
                manifest.version,
                manifest.id,
                manifest.authors.join(", "),
                permissions
            );
        }

        wrapped(&text, columns)
    }

    #[must_use]
    pub fn ui_scroll_max(&self, _page: &PluginPage, camera: &Camera) -> f32 {
        if let Some(confirm) = &self.confirmation {
            let [left, top, right, bottom] =
                dialog_rect(camera.screen_size.x, camera.screen_size.y);

            let columns = ((right - left - 40.0) / 8.0).max(1.0) as usize;

            let text = self.confirmation_body(confirm, columns);

            let content_height = text.split('\n').count() as f32 * 17.5;

            let viewport_height = (bottom - top - 132.0).max(1.0);

            return (content_height - viewport_height).max(0.0);
        }

        let count = self.entries.len();

        let content_height = count as f32 * CARD_HEIGHT + count.saturating_sub(1) as f32 * CARD_GAP;

        let viewport_height = (camera.screen_size.y - PAGE_TOP - PAGE_BOTTOM).max(1.0);

        (content_height - viewport_height).max(0.0)
    }
}

// LOL So big file TODO
