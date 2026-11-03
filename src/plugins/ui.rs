use super::manager::Confirmation;
use super::{PluginManager, UiFrame};
use crate::{Camera, Color};

const UI_MARGIN: f32 = 12.0;
const BUTTON_HEIGHT: f32 = 30.0;
const BUTTON_GAP: f32 = 6.0;
const LIST_ITEM_HEIGHT: f32 = 54.0;
const PAGE_TOP: f32 = 80.0;
const PAGE_BOTTOM: f32 = 76.0;
const CARD_HEIGHT: f32 = 232.0;
const CARD_GAP: f32 = 20.0;
const TOOLBAR_WIDTHS: [f32; 5] = [184.0, 66.0, 66.0, 100.0, 140.0];
const TOOLBAR_HEIGHT: f32 = 32.0;
const TOOLBAR_ROW_STEP: f32 = 40.0;

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
}

struct ToolbarCursor {
    x: f32,
    y: f32,
    width: f32,
}

impl ToolbarCursor {
    fn new(width: f32) -> Self {
        Self {
            x: UI_MARGIN,
            y: UI_MARGIN,
            width,
        }
    }

    fn next(&mut self, width: f32) -> [f32; 4] {
        let width = width.min((self.width - 2.0 * UI_MARGIN).max(1.0));

        if self.x > UI_MARGIN && self.x + width > self.width - UI_MARGIN {
            self.x = UI_MARGIN;
            self.y += TOOLBAR_ROW_STEP;
        }

        let rect = [self.x, self.y, self.x + width, self.y + TOOLBAR_HEIGHT];

        self.x += width + 8.0;
        rect
    }
}

fn toolbar_bottom(width: f32) -> f32 {
    let mut cursor = ToolbarCursor::new(width);
    let mut bottom = UI_MARGIN;

    for button_width in TOOLBAR_WIDTHS {
        bottom = cursor.next(button_width)[3];
    }

    bottom
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
    (label.chars().count() as f32 * 8.0 + 24.0).clamp(70.0, 200.0)
}

fn notice_columns(width: f32) -> usize {
    ((width - 24.0) / 7.0).max(20.0) as usize
}

fn sidebar_width(screen_width: f32) -> f32 {
    (screen_width * 0.30).clamp(160.0, 260.0)
}

fn content_columns(left: f32, right: f32) -> usize {
    ((right - left) / 8.0).max(16.0) as usize
}

fn confirmation_columns(left: f32, right: f32) -> usize {
    ((right - left - 32.0) / 8.0).max(20.0) as usize
}

impl PluginManager {
    #[must_use]
    pub fn ui_frame(&self, page: &PluginPage, camera: &Camera) -> UiFrame {
        let mut frame = UiFrame::with_theme(self.settings.appearance.theme());

        let screen = [camera.screen_size.x, camera.screen_size.y];

        if !page.open {
            self.draw_halloween_decoration(&mut frame, camera, screen, false);

            self.draw_toolbar(&mut frame, page, camera, screen);

            self.draw_quick_actions(&mut frame, camera, screen);

            self.draw_notice(&mut frame, camera, screen);

            return frame;
        }

        self.draw_plugin_page(&mut frame, page, camera, screen);

        if self.confirmation.is_some() {
            self.draw_confirmation_dialog(&mut frame, page, camera, screen);
        }

        frame
    }

    fn draw_toolbar(
        &self,
        frame: &mut UiFrame,
        _page: &PluginPage,
        camera: &Camera,
        [w, _h]: [f32; 2],
    ) {
        let appearance = self.settings.appearance;

        let controls = [
            ("Plugins (Ctrl+Shift+P)", UiAction::TogglePage, false),
            ("Save", UiAction::Save, false),
            ("Open", UiAction::Open, false),
            (
                if appearance.dark {
                    "Dark: on"
                } else {
                    "Dark: off"
                },
                UiAction::ToggleDark,
                appearance.dark,
            ),
            (
                if appearance.halloween {
                    "Halloween: on"
                } else {
                    "Halloween: off"
                },
                UiAction::ToggleHalloween,
                appearance.halloween,
            ),
        ];

        let mut cursor = ToolbarCursor::new(w);

        for ((label, action, selected), width) in controls.into_iter().zip(TOOLBAR_WIDTHS) {
            frame.button(camera, cursor.next(width), label, action, selected);
        }
    }

    fn draw_halloween_decoration(
        &self,
        frame: &mut UiFrame,
        _camera: &Camera,
        [w, h]: [f32; 2],
        plugin_page: bool,
    ) {
        use super::UiImageKind::{Blood, Pumpkin, Strips, Web};

        if !frame.theme.halloween || self.confirmation.is_some() || w < 360.0 || h < 260.0 {
            return;
        }

        let scale = (w / 1000.0).min(h / 700.0).clamp(0.65, 1.4);

        let padding = 12.0 * scale;

        let strip_width = (160.0 * scale).min(w * 0.24);

        let strip_height = if plugin_page { h } else { h * 0.65 };

        let web_size = (128.0 * scale).min(w * 0.24).min(h * 0.30);

        let pumpkin_size = (96.0 * scale).min(w * 0.18).min(h * 0.24);

        let pumpkin_top = if plugin_page { 64.0 } else { 20.0 * scale };

        frame.image(
            Strips,
            [w - strip_width, 0.0, w, strip_height],
            [1.0, 0.0],
            false,
        );

        if !plugin_page {
            frame.image(
                Blood,
                [
                    0.0,
                    0.0,
                    (180.0 * scale).min(w * 0.25),
                    (280.0 * scale).min(h * 0.55),
                ],
                [0.0, 0.0],
                false,
            );
        }

        frame.image(Web, [0.0, h - web_size, web_size, h], [0.0, 1.0], false);

        if !plugin_page {
            frame.image(Web, [w - web_size, h - web_size, w, h], [1.0, 1.0], true);
        }

        frame.image(
            Pumpkin,
            [
                w - padding - pumpkin_size,
                pumpkin_top,
                w - padding,
                pumpkin_top + pumpkin_size,
            ],
            [1.0, 0.0],
            false,
        );
    }

    fn draw_quick_actions(&self, frame: &mut UiFrame, camera: &Camera, [w, _h]: [f32; 2]) {
        let mut x = UI_MARGIN;
        let mut y = toolbar_bottom(w) + 8.0;

        for (plugin_id, entry) in &self.entries {
            if entry.runtime.is_none() {
                continue;
            }

            for tool in &entry.contributions.tools {
                let action = UiAction::Tool(plugin_id.clone(), tool.id.clone());
                let selected =
                    self.selected.as_ref() == Some(&(plugin_id.clone(), tool.id.clone()));

                self.draw_flow_button(
                    frame,
                    camera,
                    &mut x,
                    &mut y,
                    &tool.label,
                    action,
                    selected,
                    w,
                    &tool.style,
                );
            }

            for command in &entry.contributions.commands {
                let action = UiAction::Command(plugin_id.clone(), command.id.clone());

                self.draw_flow_button(
                    frame,
                    camera,
                    &mut x,
                    &mut y,
                    &command.label,
                    action,
                    false,
                    w,
                    &command.style,
                );
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_flow_button(
        &self,
        frame: &mut UiFrame,
        camera: &Camera,
        x: &mut f32,
        y: &mut f32,
        label: &str,
        action: UiAction,
        selected: bool,
        screen_width: f32,
        style: &super::runtime::ButtonStyle,
    ) {
        let width = text_width(label);

        if *x + width > screen_width - UI_MARGIN {
            *x = UI_MARGIN;
            *y += 36.0;
        }

        frame.button_styled(
            camera,
            [*x, *y, *x + width, *y + BUTTON_HEIGHT],
            label,
            action,
            selected,
            style,
        );

        *x += width + BUTTON_GAP;
    }

    fn draw_notice(&self, frame: &mut UiFrame, camera: &Camera, [w, h]: [f32; 2]) {
        if self.notice.is_empty() {
            return;
        }

        let text = clipped(&self.notice, notice_columns(w));
        let theme = self.settings.appearance.theme();

        frame.text(camera, UI_MARGIN, h - 38.0, 12.0, text, theme.warning);
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

            text.push_str(&format!(
                "\n\n{} v{}\n{}\nAuthors: {}\nPermissions: {}",
                manifest.name,
                manifest.version,
                manifest.id,
                manifest.authors.join(", "),
                permissions,
            ));
        }

        wrapped(&text, columns)
    }

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
