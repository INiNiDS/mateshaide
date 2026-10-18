use crate::canvas::Canvas;
use crate::color::Color;
use crate::palette;
use crate::shape::ShapeInstance;
use glam::Vec2;

const TEST_PALETTE: &[Color] = &[
    palette::RED_500,
    palette::GREEN_500,
    palette::BLUE_500,
    palette::VIOLET_500,
    palette::AMBER_500,
    palette::CYAN_500,
    palette::PINK_500,
    palette::TEAL_500,
];

#[derive(Debug, Clone, Copy)]
pub struct TestShapeSpec {
    pub name: &'static str,
    pub kind: u32,
}

impl TestShapeSpec {
    pub const fn new(name: &'static str, kind: u32) -> TestShapeSpec {
        Self { name, kind }
    }
}

#[derive(Default)]
pub struct TestSpawner {
    pub active: bool,
    pub current: Option<TestShapeSpec>,
    pub cursor_screen: Vec2,
    pub color_mode: ColorMode,
    palette_cursor: usize,
}

impl TestSpawner {
    pub fn toggle(&mut self) {
        self.active = !self.active;
        tracing::info!(
            "test spawn mode: {}",
            if self.active { "on" } else { "off" }
        );
    }

    pub fn select(&mut self, name: &'static str, kind: u32) {
        self.current = Some(TestShapeSpec::new(name, kind));
        tracing::info!("test spawn selected: {name}, (kind={kind})");
    }

    pub fn spawn(&mut self, canvas: &mut Canvas) {
        let Some(spec) = self.current else {
            tracing::warn!("test spawn: not shape selected");
            return;
        };

        let world = canvas.camera.screen_to_world(self.cursor_screen);
        let color = TEST_PALETTE[self.palette_cursor % TEST_PALETTE.len()];
        self.palette_cursor = self.palette_cursor.wrapping_add(1);

        canvas.add_shape(build_instance(spec, world, color));
    }
}

fn build_instance(spec: TestShapeSpec, world: Vec2, color: Color) -> ShapeInstance {
    match spec.name {
        // SDF rect: a.xy — half-extents, quad size = 2 * half-extents.
        "rect" => ShapeInstance::new(spec.kind, world, Vec2::new(80.0, 60.0), color)
            .with_params([40.0, 30.0, 0.0, 0.0], [0.0; 4]),

        // SDF circle: a.x — radius.
        "circle" => ShapeInstance::new(spec.kind, world, Vec2::splat(70.0), color)
            .with_params([35.0, 0.0, 0.0, 0.0], [0.0; 4]),

        // SDF hexagon: a.x — circumradius.
        "hexagon" => ShapeInstance::new(spec.kind, world, Vec2::splat(80.0), color)
            .with_params([35.0, 0.0, 0.0, 0.0], [0.0; 4]),

        // SDF rounded_rect: a.xy — half-extents, a.z — corner radius.
        "rounded_rect" => ShapeInstance::new(spec.kind, world, Vec2::new(80.0, 60.0), color)
            .with_params([40.0, 30.0, 12.0, 0.0], [0.0; 4]),

        _ => ShapeInstance::new(spec.kind, world, Vec2::splat(60.0), color),
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorMode {
    #[default]
    Cycle,
    Fixed
}

impl ColorMode {
    fn toggle(&self) -> Self {
        match self {
            ColorMode::Cycle => ColorMode::Fixed,
            ColorMode::Fixed => ColorMode::Cycle,
        }
    }
}

