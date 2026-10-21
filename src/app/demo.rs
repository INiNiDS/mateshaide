use crate::core::Color;
use crate::engine::Engine;
use crate::palette;
use crate::shape::ShapeInstance;
use crate::canvas::CanvasText;
use glam::Vec2;

pub(super) fn seed_demo_shapes(engine: &mut Engine) {
    engine.canvas.add_shape(ShapeInstance::rect(
        -50.0,
        -50.0,
        100.0,
        100.0,
        palette::RED_500,
    ));

    engine.canvas.add_shape(ShapeInstance::rect(
        120.0,
        60.0,
        180.0,
        90.0,
        Color::hex(0x22c55e),
    ));

    engine.canvas.add_shape(ShapeInstance::rect(
        -300.0,
        -150.0,
        80.0,
        300.0,
        palette::BLUE_500,
    ));

    engine.canvas.add_shape(ShapeInstance::rect(
        200.0,
        -200.0,
        60.0,
        60.0,
        palette::VIOLET_500,
    ));

    engine
        .canvas
        .add_shape(ShapeInstance::circle(0.0, 200.0, 40.0, palette::INFO));

    engine.canvas.add_text(CanvasText {
        position: Vec2::new(-50.0, -80.0),
        text: "Hello, MateshaIDE!".to_owned(),
        size: 24.0,
        color: palette::GRAY_900,
    });
}
