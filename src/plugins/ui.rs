use super::manager::Confirmation;
use super::{PluginManager, UiFrame};
use crate::{Camera, Color};

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
}

#[derive(Default)]
pub struct PluginPage {
    pub open: bool,
    pub selected: Option<String>,
    pub scroll: f32,
}

const UI_MARGIN: f32 = 12.0;
const BUTTON_HEIGHT: f32 = 30.0;
const BUTTON_GAP: f32 = 6.0;
const LIST_ITEM_HEIGHT: f32 = 54.0;

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

    for ch in s.chars().take(1600) {
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
    pub fn ui_frame(&self, page: &PluginPage, camera: &Camera) -> UiFrame {
        let mut frame = UiFrame::default();
        let screen = [camera.screen_size.x, camera.screen_size.y];

        if !page.open {
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
        frame.button(
            camera,
            [12.0, 12.0, 196.0, 44.0],
            "Plugins (Ctrl+Shift+P)",
            UiAction::TogglePage,
            false,
        );

        frame.button(
            camera,
            [204.0, 12.0, 270.0, 44.0],
            "Save",
            UiAction::Save,
            false,
        );

        frame.button(
            camera,
            [278.0, 12.0, 344.0, 44.0],
            "Open",
            UiAction::Open,
            false,
        );
    }

    fn draw_quick_actions(&self, frame: &mut UiFrame, camera: &Camera, [w, _h]: [f32; 2]) {
        let mut x = UI_MARGIN;
        let mut y = 52.0;

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
    ) {
        let width = text_width(label);

        if *x + width > screen_width - UI_MARGIN {
            *x = UI_MARGIN;
            *y += 36.0;
        }

        frame.button(
            camera,
            [*x, *y, *x + width, *y + BUTTON_HEIGHT],
            label,
            action,
            selected,
        );

        *x += width + BUTTON_GAP;
    }

    fn draw_notice(&self, frame: &mut UiFrame, camera: &Camera, [w, h]: [f32; 2]) {
        if self.notice.is_empty() {
            return;
        }

        let text = clipped(&self.notice, notice_columns(w));

        frame.text(
            camera,
            UI_MARGIN,
            h - 38.0,
            12.0,
            text,
            Color::rgb(1.0, 0.72, 0.36),
        );
    }

    fn draw_plugin_page(
        &self,
        frame: &mut UiFrame,
        page: &PluginPage,
        camera: &Camera,
        [w, h]: [f32; 2],
    ) {
        frame.rect([0.0, 0.0, w, h], Color::rgb(0.065, 0.075, 0.095), w, h);

        self.draw_page_header(frame, page, camera, w);
        self.draw_plugin_list(frame, page, camera, w, h);

        let sidebar = sidebar_width(w);

        if let Some((plugin_id, entry)) = self.selected_or_first(page) {
            self.draw_plugin_details(frame, page, camera, plugin_id, entry, w, h, sidebar);
        }

        self.draw_page_footer(frame, camera, w, h);
    }

    fn draw_page_header(&self, frame: &mut UiFrame, _page: &PluginPage, camera: &Camera, w: f32) {
        frame.text(camera, 22.0, 18.0, 24.0, "Plugins", Color::WHITE);

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

    fn draw_plugin_list(
        &self,
        frame: &mut UiFrame,
        page: &PluginPage,
        camera: &Camera,
        w: f32,
        h: f32,
    ) {
        let sidebar = sidebar_width(w);
        let mut y = 66.0 - page.scroll;

        for (plugin_id, entry) in &self.entries {
            let visible = y >= 54.0 && y + 50.0 < h - 68.0;

            if visible {
                let label = format!(
                    "{} {}",
                    if entry.runtime.is_some() {
                        "[on]"
                    } else {
                        "[off]"
                    },
                    clipped(&entry.package.manifest.name, 22),
                );

                let selected = page.selected.as_deref() == Some(plugin_id.as_str());

                frame.button(
                    camera,
                    [UI_MARGIN, y, sidebar - 10.0, y + 46.0],
                    &label,
                    UiAction::SelectPlugin(plugin_id.clone()),
                    selected,
                );
            }

            y += LIST_ITEM_HEIGHT;
        }
    }

    fn selected_or_first<'a>(
        &'a self,
        page: &'a PluginPage,
    ) -> Option<(&'a String, &'a super::manager::Entry)> {
        page.selected
            .as_ref()
            .and_then(|id| self.entries.get_key_value(id))
            .or_else(|| self.entries.iter().next())
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_plugin_details(
        &self,
        frame: &mut UiFrame,
        page: &PluginPage,
        camera: &Camera,
        plugin_id: &str,
        entry: &super::manager::Entry,
        w: f32,
        h: f32,
        sidebar: f32,
    ) {
        let manifest = &entry.package.manifest;
        let x = sidebar + 14.0;
        let right = (w - 20.0).max(x + 100.0);
        let columns = content_columns(x, right);

        self.draw_plugin_icon(frame, camera, entry, x, w, h);

        frame.text(
            camera,
            x + 60.0,
            66.0,
            21.0,
            clipped(&manifest.name, 45),
            Color::WHITE,
        );

        frame.text(
            camera,
            x + 60.0,
            94.0,
            13.0,
            format!("{}  v{}", manifest.id, manifest.version),
            Color::rgb(0.65, 0.75, 0.85),
        );

        let mut y = self.draw_plugin_info(frame, camera, manifest, x, right, columns);
        y = self.draw_plugin_actions(frame, camera, plugin_id, entry, x, right, y);
        y = self.draw_dependencies(frame, camera, manifest, x, right, columns, y, h);

        self.draw_plan_error(frame, camera, plugin_id, x, columns, y, h);
    }

    fn draw_plugin_icon(
        &self,
        frame: &mut UiFrame,
        camera: &Camera,
        entry: &super::manager::Entry,
        x: f32,
        w: f32,
        h: f32,
    ) {
        let Some(icon) = &entry.package.icon else {
            frame.rect(
                [x, 66.0, x + 48.0, 114.0],
                Color::rgb(0.2, 0.36, 0.52),
                w,
                h,
            );
            return;
        };

        for (row, icon_row) in icon.rows().enumerate() {
            for (column, pixel) in icon_row.enumerate() {
                if pixel[3] == 0 {
                    continue;
                }

                let px = x + column as f32 * 2.0;
                let py = 66.0 + row as f32 * 2.0;

                frame.rect(
                    [px, py, px + 2.1, py + 2.1],
                    Color::rgba8(pixel[0], pixel[1], pixel[2], pixel[3]),
                    w,
                    h,
                );
            }
        }
    }

    fn draw_plugin_info(
        &self,
        frame: &mut UiFrame,
        camera: &Camera,
        manifest: &super::manifest::Manifest,
        x: f32,
        right: f32,
        columns: usize,
    ) -> f32 {
        let description = wrapped(&manifest.description, columns);
        let description_height = description.lines().count() as f32 * 18.0;

        frame.text(camera, x, 130.0, 14.0, description, Color::WHITE);

        let mut y = 140.0 + description_height;

        let info_lines = [
            format!("Authors: {}", manifest.authors.join(", ")),
            format!("API: {}", manifest.api),
            format!(
                "Permissions: {}",
                if manifest.permissions.is_empty() {
                    "none".into()
                } else {
                    manifest.permissions.join(", ")
                }
            ),
        ];

        for line in info_lines {
            let line = wrapped(&line, columns);
            let line_height = line.lines().count() as f32 * 17.0;

            frame.text(camera, x, y, 13.0, line, Color::rgb(0.74, 0.8, 0.88));

            y += line_height + 7.0;
        }

        let _ = right;
        y
    }

    fn draw_plugin_actions(
        &self,
        frame: &mut UiFrame,
        camera: &Camera,
        plugin_id: &str,
        entry: &super::manager::Entry,
        x: f32,
        right: f32,
        mut y: f32,
    ) -> f32 {
        let toggle_label = if entry.runtime.is_some() {
            "Disable"
        } else {
            "Activate"
        };

        frame.button(
            camera,
            [x, y, (x + 150.0).min(right), y + 32.0],
            toggle_label,
            UiAction::TogglePlugin(plugin_id.to_string()),
            false,
        );

        frame.button(
            camera,
            [(x + 160.0).min(right - 90.0), y, right, y + 32.0],
            "Repository",
            UiAction::Repository(entry.package.manifest.repository.clone()),
            false,
        );

        y += 44.0;

        frame.text(
            camera,
            x,
            y,
            12.0,
            clipped(
                &entry.package.manifest.repository,
                content_columns(x, right),
            ),
            Color::rgb(0.45, 0.72, 1.0),
        );

        y + 24.0
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_dependencies(
        &self,
        frame: &mut UiFrame,
        camera: &Camera,
        manifest: &super::manifest::Manifest,
        x: f32,
        right: f32,
        columns: usize,
        mut y: f32,
        h: f32,
    ) -> f32 {
        if manifest.dependencies.is_empty() {
            frame.text(camera, x, y, 13.0, "Dependencies: none", Color::WHITE);
            return y + 24.0;
        }

        frame.text(camera, x, y, 13.0, "Dependencies:", Color::WHITE);
        y += 22.0;

        for dependency in &manifest.dependencies {
            let satisfied = self.dependency_satisfied(dependency);
            let status = if satisfied {
                "installed"
            } else {
                "missing/incompatible"
            };

            let label = clipped(
                &format!("{} {} [{}]", dependency.id, dependency.version, status),
                columns,
            );

            let color = if satisfied {
                Color::rgb(0.5, 0.85, 0.62)
            } else {
                Color::rgb(1.0, 0.6, 0.3)
            };

            frame.text(camera, x, y, 12.0, label, color);
            y += 20.0;

            if y + 28.0 < h - 92.0 {
                frame.button(
                    camera,
                    [x, y, right, y + 28.0],
                    &clipped(&dependency.repository, columns),
                    UiAction::Repository(dependency.repository.clone()),
                    false,
                );

                y += 35.0;
            }
        }

        y
    }

    fn dependency_satisfied(&self, dependency: &super::manifest::Dependency) -> bool {
        let Some(entry) = self.entries.get(&dependency.id) else {
            return false;
        };

        let Ok(installed) = semver::Version::parse(&entry.package.manifest.version) else {
            return false;
        };

        let Ok(requirement) = semver::VersionReq::parse(&dependency.version) else {
            return false;
        };

        requirement.matches(&installed)
    }

    fn draw_plan_error(
        &self,
        frame: &mut UiFrame,
        camera: &Camera,
        plugin_id: &str,
        x: f32,
        columns: usize,
        y: f32,
        h: f32,
    ) {
        let Ok(()) = self.plan(plugin_id) else {
            return;
        };

        let error = match self.plan(plugin_id) {
            Err(error) => error,
            Ok(()) => return,
        };

        if y + 40.0 >= h - 68.0 {
            return;
        }

        let text = wrapped(&format!("Blocked: {error:#}"), columns);

        frame.text(camera, x, y + 24.0, 12.0, text, Color::rgb(1.0, 0.6, 0.3));
    }

    fn draw_page_footer(&self, frame: &mut UiFrame, camera: &Camera, w: f32, h: f32) {
        frame.text(
            camera,
            14.0,
            h - 58.0,
            12.0,
            clipped(&self.notice, notice_columns(w)),
            Color::rgb(1.0, 0.75, 0.4),
        );

        frame.text(
            camera,
            14.0,
            h - 30.0,
            12.0,
            "Install: clone/unpack into plugins/, then Rescan. Browser: use Import ZIP in its build.",
            Color::rgb(0.6, 0.68, 0.78),
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

        frame.rect([0.0, 0.0, w, h], Color::new(0.0, 0.0, 0.0, 0.83), w, h);
        frame.hits.clear();

        let left = 24.0;
        let top = 50.0;
        let right = w - 24.0;

        frame.rect(
            [left, top, right, h - 40.0],
            Color::rgb(0.11, 0.13, 0.17),
            w,
            h,
        );

        let (title, plugin_ids) = match confirm {
            Confirmation::Enable(ids) => ("Security: activate plugins?", ids),
            Confirmation::Disable(ids) => ("Disable plugins and their dependents?", ids),
        };

        frame.text(camera, left + 16.0, top + 14.0, 20.0, title, Color::WHITE);

        let columns = confirmation_columns(left, right);
        let warning = self.confirmation_warning(confirm);
        let warning = wrapped(warning, columns);
        let warning_height = warning.lines().count() as f32 * 18.0;

        frame.text(
            camera,
            left + 16.0,
            top + 50.0,
            14.0,
            warning,
            Color::rgb(1.0, 0.77, 0.47),
        );

        let list_top = top + 58.0 + warning_height;
        self.draw_confirmation_plugins(
            frame,
            camera,
            plugin_ids,
            columns,
            list_top,
            page.scroll,
            h,
        );

        self.draw_confirmation_footer(frame, camera, plugin_ids.len(), left, h);
    }

    fn confirmation_warning<'a>(&self, confirm: &'a Confirmation) -> &'a str {
        if matches!(confirm, Confirmation::Enable(_)) {
            "Plugin code runs after confirmation. Requested permissions allow changes to your document or clipboard. Sandboxing is not a guarantee against all bugs or resource exhaustion. Activate only code you trust. Dependencies listed below will also be activated."
        } else {
            "Dependent plugins must stop too. Created objects remain in the document and will show placeholders until their plugins are enabled again."
        }
    }

    fn draw_confirmation_plugins(
        &self,
        frame: &mut UiFrame,
        camera: &Camera,
        plugin_ids: &[String],
        columns: usize,
        list_top: f32,
        scroll: f32,
        h: f32,
    ) {
        let left = 24.0;
        let mut y = list_top - scroll;

        for plugin_id in plugin_ids {
            let entry = &self.entries[plugin_id];
            let manifest = &entry.package.manifest;

            let text = wrapped(
                &format!(
                    "{} v{} | {} | {}",
                    plugin_id,
                    manifest.version,
                    manifest.authors.join(", "),
                    manifest.permissions.join(", "),
                ),
                columns,
            );

            let item_height = text.lines().count() as f32 * 16.0 + 12.0;
            let visible = y >= list_top && y + item_height < h - 110.0;

            if visible {
                frame.text(camera, left + 16.0, y + 10.0, 12.0, &text, Color::WHITE);
            }

            y += item_height;
        }
    }

    fn draw_confirmation_footer(
        &self,
        frame: &mut UiFrame,
        camera: &Camera,
        plugin_count: usize,
        left: f32,
        h: f32,
    ) {
        frame.text(
            camera,
            left + 16.0,
            h - 120.0,
            12.0,
            format!("{plugin_count} plugins in this operation. Scroll to review."),
            Color::rgb(0.7, 0.8, 0.9),
        );

        frame.button(
            camera,
            [left + 16.0, h - 94.0, left + 170.0, h - 58.0],
            "Confirm",
            UiAction::Confirm,
            false,
        );

        frame.button(
            camera,
            [left + 180.0, h - 94.0, left + 310.0, h - 58.0],
            "Cancel",
            UiAction::Cancel,
            false,
        );
    }
}
