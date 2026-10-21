use crate::canvas::Canvas;
use crate::core::Color;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestMode {
    Spawn,
    Erase,
}

impl TestShapeSpec {
    pub const fn new(name: &'static str, kind: u32) -> TestShapeSpec {
        Self { name, kind }
    }
}

#[derive(Default)]
pub struct TestSpawner {
    pub mode: Option<TestMode>,
    pub current: Option<TestShapeSpec>,
    pub cursor_screen: Vec2,
    pub color_mode: ColorMode,
    pub picked_color: Color,
    palette_cursor: usize,
}

impl TestSpawner {
    pub fn toggle_mode(&mut self, target: TestMode) {
        if self.mode == Some(target) {
            self.mode = None;
        } else {
            self.mode = Some(target);
        }
        tracing::info!("test mode: {:?}", self.mode);
    }

    pub fn select(&mut self, name: &'static str, kind: u32) {
        self.current = Some(TestShapeSpec::new(name, kind));
        tracing::info!("test spawn selected: {name}, (kind={kind})");
    }

    pub fn spawn(&mut self, canvas: &mut Canvas) {
        if !matches!(self.mode, Some(TestMode::Spawn)) {
            return;
        }
        let Some(spec) = self.current else {
            tracing::warn!("test spawn: not shape selected");
            return;
        };

        let world = canvas.camera.screen_to_world(self.cursor_screen);
        let color = self.next_color(spec);
        self.palette_cursor = self.palette_cursor.wrapping_add(1);

        canvas.add_shape(build_instance(spec, world, color));
    }

    pub fn next_picked_color(&mut self) {
        let idx = TEST_PALETTE
            .iter()
            .position(|&c| c == self.picked_color)
            .map_or(0, |i| (i + 1) % TEST_PALETTE.len());
        self.picked_color = TEST_PALETTE[idx];
        tracing::info!("test picked color: {:?}", self.picked_color);
    }

    pub fn toggle_color_mode(&mut self) {
        self.color_mode = self.color_mode.cycle();
        tracing::info!("test color mode: {:?}", self.color_mode);
    }

    pub fn preview_color(&self) -> Option<Color> {
        if !matches!(self.mode, Some(TestMode::Spawn)) {
            return None;
        }
        let spec = self.current?;
        Some(match self.color_mode {
            ColorMode::Cycle => TEST_PALETTE[self.palette_cursor % TEST_PALETTE.len()],
            ColorMode::Fixed => fixed_color(spec),
            ColorMode::Picked => self.picked_color,
        })
    }

    pub fn erase_at_cursor(&mut self, canvas: &mut Canvas) -> bool {
        if !matches!(self.mode, Some(TestMode::Erase)) {
            return false;
        }
        let world = canvas.camera.screen_to_world(self.cursor_screen);
        let erased = canvas.erase_at_world(world);
        if erased {
            tracing::info!("test erase: shape erased at cursor");
        } else {
            tracing::info!("test erase: no shape found at cursor");
        }
        erased
    }

    fn next_color(&mut self, spec: TestShapeSpec) -> Color {
        match self.color_mode {
            ColorMode::Cycle => {
                let color = TEST_PALETTE[self.palette_cursor % TEST_PALETTE.len()];
                self.palette_cursor = self.palette_cursor.wrapping_add(1);
                color
            }
            ColorMode::Fixed => fixed_color(spec),
            ColorMode::Picked => self.picked_color,
        }
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
    Fixed,
    Picked,
}

impl ColorMode {
    fn cycle(&self) -> Self {
        match self {
            ColorMode::Cycle => ColorMode::Fixed,
            ColorMode::Fixed => ColorMode::Picked,
            ColorMode::Picked => ColorMode::Cycle,
        }
    }
}

fn fixed_color(spec: TestShapeSpec) -> Color {
    match spec.name {
        "rect" => palette::BLUE_500,
        "circle" => palette::GREEN_500,
        "hexagon" => palette::VIOLET_500,
        "rounded_rect" => palette::AMBER_500,
        _ => palette::ACCENT,
    }
}
