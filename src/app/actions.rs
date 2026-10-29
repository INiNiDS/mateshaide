use crate::engine::Engine;
use crate::export::ExportFormat;
use crate::input::{InputManager, Shortcut};
use winit::keyboard::KeyCode;

pub(super) fn register(input: &mut InputManager) {
    register_draw_actions(input);
    register_history_actions(input);
    register_camera_actions(input);
    register_export_actions(input);
}

fn register_draw_actions(input: &mut InputManager) {
    input.register_action("toggle_draw", |engine: &mut Engine| {
        engine.clear_plugin_tool();
        engine.draw.end(&mut engine.canvas);
        #[cfg(debug_assertions)]
        if !engine.draw.enabled {
            engine.disable_test_tools();
        }
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
        #[cfg(debug_assertions)]
        engine.finish_test_interaction();
        engine.canvas.zoom_at_center(1.0);
    });

    input.register_action("zoom_out", |engine: &mut Engine| {
        #[cfg(debug_assertions)]
        engine.finish_test_interaction();
        engine.canvas.zoom_at_center(-1.0);
    });

    input.register_action("reset_view", |engine: &mut Engine| {
        #[cfg(debug_assertions)]
        engine.finish_test_interaction();
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

fn register_export_actions(input: &mut InputManager) {
    input.register_action("export_png", |engine: &mut Engine| {
        engine.begin_png_capture();
    });

    input.register_action("export_pdf", |engine: &mut Engine| {
        crate::export::export_to_file(engine, ExportFormat::Pdf);
    });

    input.bind(Shortcut::key(KeyCode::KeyE).ctrl(), "export_png");
    input.bind(Shortcut::key(KeyCode::KeyP).ctrl(), "export_pdf");
}
