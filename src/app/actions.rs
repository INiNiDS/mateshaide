use crate::engine::Engine;
use crate::input::{InputManager, Shortcut};
use winit::keyboard::KeyCode;
use crate::export::ExportFormat;

pub(super) fn register(input: &mut InputManager) {
    register_draw_actions(input);
    register_history_actions(input);
    register_camera_actions(input);
    #[cfg(not(target_arch = "wasm32"))]
    register_export_actions(input);
}

fn register_draw_actions(input: &mut InputManager) {
    input.register_action("toggle_draw", |engine: &mut Engine| {
        engine.draw.toggle();
    });

    input.bind(Shortcut::key(KeyCode::Digit9).ctrl(), "toggle_draw");
}

fn register_history_actions(input: &mut InputManager) {
    input.register_action("undo", |engine: &mut Engine| {
        engine.undo();
    });

    input.register_action("redo", |engine: &mut Engine| {
        engine.redo();
    });

    input.bind(Shortcut::key(KeyCode::KeyZ).ctrl(), "undo");

    input.bind(Shortcut::key(KeyCode::KeyX).ctrl(), "redo");

    input.bind(Shortcut::key(KeyCode::KeyY).ctrl(), "redo");

    input.bind(Shortcut::key(KeyCode::KeyZ).ctrl().shift(), "redo");
}

fn register_camera_actions(input: &mut InputManager) {
    input.register_action("zoom_in", |engine: &mut Engine| {
        engine.canvas.zoom_at_center(1.0);
    });

    input.register_action("zoom_out", |engine: &mut Engine| {
        engine.canvas.zoom_at_center(-1.0);
    });

    input.register_action("reset_view", |engine: &mut Engine| {
        engine.canvas.reset_view();
    });

    input.bind(Shortcut::key(KeyCode::Equal).ctrl(), "zoom_in");

    input.bind(Shortcut::key(KeyCode::NumpadAdd).ctrl(), "zoom_in");

    input.bind(Shortcut::key(KeyCode::Equal).ctrl().shift(), "zoom_in");

    input.bind(Shortcut::key(KeyCode::Minus).ctrl(), "zoom_out");

    input.bind(Shortcut::key(KeyCode::NumpadSubtract).ctrl(), "zoom_out");

    input.bind(Shortcut::key(KeyCode::Digit0).ctrl(), "reset_view");

    input.bind(Shortcut::key(KeyCode::Numpad0).ctrl(), "reset_view");
}

#[cfg(not(target_arch = "wasm32"))]
fn register_export_actions(input: &mut InputManager) {
    input.register_action("export_png", |engine: &mut Engine| {
        crate::export::export_to_file(engine, ExportFormat::Png);
    });

    input.register_action("export_pdf", |engine: &mut Engine| {
        crate::export::export_to_file(engine, ExportFormat::Pdf);
    });

    input.bind(Shortcut::key(KeyCode::KeyE).ctrl(), "export_png");
    input.bind(Shortcut::key(KeyCode::KeyP).ctrl(), "export_pdf");
}
