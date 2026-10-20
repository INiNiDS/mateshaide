use crate::engine::Engine;
use crate::input::{InputManager, Shortcut};
use winit::keyboard::KeyCode;

pub fn initialize_input(input: &mut InputManager) {
    bind_test_features(input);
    bind_test_color_features(input);
    bind_camera_features(input);
    bind_history_features(input);
    bind_other_features(input);
}

fn bind_other_features(input: &mut InputManager) {
     input.register_action("toggle_draw", |engine: &mut Engine| {
        engine.draw.toggle();
    });
    input.bind(Shortcut::key(KeyCode::Digit9).ctrl(), "toggle_draw");
}

fn bind_history_features(input: &mut InputManager) {
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

fn bind_test_features(input: &mut InputManager) {
    input.register_action("test_toggle_spawn", |engine: &mut Engine| {
        engine.toggle_test_spawn();
    });
    input.register_action("test_select_rect", |engine: &mut Engine| {
        engine.select_test_shape("rect");
    });
    input.register_action("test_select_circle", |engine: &mut Engine| {
        engine.select_test_shape("circle");
    });
    input.register_action("test_select_hexagon", |engine: &mut Engine| {
        engine.select_test_shape("hexagon");
    });
    input.register_action("test_select_rounded_rect", |engine: &mut Engine| {
        engine.select_test_shape("rounded_rect");
    });
    input.register_action("test_spawn", |engine: &mut Engine| {
        engine.spawn_test_shape();
    });
    input.bind(Shortcut::key(KeyCode::Digit1).ctrl(), "test_toggle_spawn");
    input.bind(Shortcut::key(KeyCode::Digit2).ctrl(), "test_select_rect");
    input.bind(Shortcut::key(KeyCode::Digit3).ctrl(), "test_select_circle");
    input.bind(Shortcut::key(KeyCode::Digit4).ctrl(), "test_select_hexagon");
    input.bind(
        Shortcut::key(KeyCode::Digit5).ctrl(),
        "test_select_rounded_rect",
    );
    input.bind(Shortcut::key(KeyCode::Digit1), "test_spawn");
    input.register_action("test_toggle_eraser", |engine: &mut Engine| {
        engine.toggle_test_eraser();
    });
    input.bind(Shortcut::key(KeyCode::Digit8).ctrl(), "test_toggle_eraser");
}

fn bind_test_color_features(input: &mut InputManager) {
    input.register_action("test_toggle_color_mode", |engine: &mut Engine| {
        engine.toggle_test_color_mode();
    });
    input.bind(
        Shortcut::key(KeyCode::Digit6).ctrl(),
        "test_toggle_color_mode",
    );

    input.register_action("test_next_picked_color", |engine: &mut Engine| {
        engine.next_test_picked_color();
    });
    input.bind(
        Shortcut::key(KeyCode::Digit7).ctrl(),
        "test_next_picked_color",
    );
}

fn bind_camera_features(input: &mut InputManager) {
    input.register_action("zoom_in", |engine: &mut Engine| {
        engine.canvas.zoom_at_center(1.0)
    });
    input.register_action("zoom_out", |engine: &mut Engine| {
        engine.canvas.zoom_at_center(-1.0)
    });
    input.register_action("reset_view", |engine: &mut Engine| {
        engine.canvas.reset_view()
    });

    input.bind(Shortcut::key(KeyCode::Equal).ctrl(), "zoom_in");
    input.bind(Shortcut::key(KeyCode::NumpadAdd).ctrl(), "zoom_in");
    input.bind(Shortcut::key(KeyCode::Equal).ctrl().shift(), "zoom_in");

    input.bind(Shortcut::key(KeyCode::Minus).ctrl(), "zoom_out");
    input.bind(Shortcut::key(KeyCode::NumpadSubtract).ctrl(), "zoom_out");

    input.bind(Shortcut::key(KeyCode::Digit0).ctrl(), "reset_view");
    input.bind(Shortcut::key(KeyCode::Numpad0).ctrl(), "reset_view");
}
