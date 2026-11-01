use crate::engine::Engine;
use crate::input::{InputManager, Shortcut};
use winit::keyboard::KeyCode;
use glam::Vec2;

pub(crate) fn register(input: &mut InputManager) {
    register_test_shape_actions(input);
    register_test_color_actions(input);
    register_test_edit_actions(input);
    register_test_size_actions(input);
}

fn register_test_shape_actions(input: &mut InputManager) {
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

    input.register_action("test_toggle_eraser", |engine: &mut Engine| {
        engine.toggle_test_eraser();
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

    input.bind(Shortcut::key(KeyCode::Digit8).ctrl(), "test_toggle_eraser");
}

fn register_test_color_actions(input: &mut InputManager) {
    input.register_action("test_toggle_color_mode", |engine: &mut Engine| {
        engine.toggle_test_color_mode();
    });

    input.register_action("test_next_picked_color", |engine: &mut Engine| {
        engine.next_test_picked_color();
    });

    input.bind(
        Shortcut::key(KeyCode::Digit6).ctrl(),
        "test_toggle_color_mode",
    );

    input.bind(
        Shortcut::key(KeyCode::Digit7).ctrl(),
        "test_next_picked_color",
    );
}

fn register_test_edit_actions(input: &mut InputManager) {
    input.register_action("test_toggle_select", |engine: &mut Engine| {
        engine.toggle_test_selection();
    });
    input.register_action("test_delete_selected", |engine: &mut Engine| {
        engine.delete_test_selection();
    });
    input.register_action("test_escape", |engine: &mut Engine| {
        engine.escape_test_interaction();
    });
    input.bind(Shortcut::key(KeyCode::KeyV), "test_toggle_select");
    input.bind(Shortcut::key(KeyCode::Digit1).ctrl().shift(), "test_toggle_select");
    input.bind(Shortcut::key(KeyCode::Delete), "test_delete_selected");
    input.bind(Shortcut::key(KeyCode::Backspace), "test_delete_selected");
    input.bind(Shortcut::key(KeyCode::Escape), "test_escape");
}

fn register_test_size_actions(input: &mut InputManager) {
    for (key, factor, all, width, height) in [
        (KeyCode::BracketLeft, 1.0 / 1.1,
         "test_size_down", "test_width_down", "test_height_down"),
        (KeyCode::BracketRight, 1.1,
         "test_size_up", "test_width_up", "test_height_up"),
    ] {
        input.register_action(all, move |engine: &mut Engine| {
            engine.scale_test_spawn_axes(Vec2::splat(factor));
        });
        input.register_action(width, move |engine: &mut Engine| {
            engine.scale_test_spawn_axes(Vec2::new(factor, 1.0));
        });
        input.register_action(height, move |engine: &mut Engine| {
            engine.scale_test_spawn_axes(Vec2::new(1.0, factor));
        });
        input.bind(Shortcut::key(key), all);
        input.bind(Shortcut::key(key).shift(), width);
        input.bind(Shortcut::key(key).alt(), height);
    }
}
