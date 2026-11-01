use super::geometry::{self, CORNERS, Corner};
use crate::canvas::Canvas;
use crate::core::Color;
use crate::palette;
use crate::shape::{ShapeInstance, ShapeRegistry};
use glam::Vec2;

const DRAG_THRESHOLD_PX: f32 = 4.0;
const HANDLE_HIT_PX: f32 = 7.0;
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
    pub const fn new(name: &'static str, kind: u32) -> Self {
        Self { name, kind }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestMode {
    Spawn,
    Select,
    Erase,
}

#[derive(Debug, Clone, Copy)]
enum TestDrag {
    Spawn {
        anchor_world: Vec2,
        anchor_screen: Vec2,
        cursor_world: Vec2,
        dragged: bool,
        square: bool,
    },
    Move {
        index: usize,
        original: ShapeInstance,
        anchor_world: Vec2,
        anchor_screen: Vec2,
    },
    Resize {
        index: usize,
        original: ShapeInstance,
        corner: Corner,
        grab_offset: Vec2,
        anchor_screen: Vec2,
    },
}

pub struct TestSpawner {
    pub mode: Option<TestMode>,
    pub current: Option<TestShapeSpec>,
    pub cursor_screen: Vec2,
    pub color_mode: ColorMode,
    pub picked_color: Color,
    pub spawn_size: Vec2,
    selected: Option<usize>,
    drag: Option<TestDrag>,
    palette_cursor: usize,
}

impl Default for TestSpawner {
    fn default() -> Self {
        Self {
            mode: None,
            current: None,
            cursor_screen: Vec2::ZERO,
            color_mode: ColorMode::default(),
            picked_color: TEST_PALETTE[0],
            spawn_size: Vec2::new(80.0, 60.0),
            selected: None,
            drag: None,
            palette_cursor: 0,
        }
    }
}

impl TestSpawner {
    pub fn toggle_mode(&mut self, target: TestMode) {
        self.mode = if self.mode == Some(target) { None } else { Some(target) };
        self.selected = None;
        tracing::info!("test mode: {:?}", self.mode);
    }

    pub fn select(&mut self, name: &'static str, kind: u32) {
        self.current = Some(TestShapeSpec::new(name, kind));
        tracing::info!("test spawn selected: {name}, kind={kind}");
    }

    pub fn set_spawn_size(&mut self, size: Vec2) {
        if size.is_finite() {
            self.spawn_size = geometry::clamp_size(size);
            tracing::info!("test spawn size: {:?}", self.spawn_size);
        }
    }

    pub fn scale_spawn_size(&mut self, factor: f32) {
        self.scale_spawn_axes(Vec2::splat(factor));
    }

    pub fn scale_spawn_axes(&mut self, factor: Vec2) {
        if factor.is_finite() && factor.min_element() > 0.0 {
            self.set_spawn_size(self.spawn_size * factor);
        }
    }

    pub fn selected(&self) -> Option<usize> {
        self.selected
    }

    pub fn clear_selection(&mut self) {
        self.selected = None;
    }

    pub fn is_dragging(&self) -> bool {
        self.drag.is_some()
    }

    pub fn pointer_pressed(
        &mut self,
        canvas: &mut Canvas,
        registry: &ShapeRegistry,
        screen: Vec2,
        shift: bool,
    ) -> bool {
        self.cursor_screen = screen;
        let Some(mode) = self.mode else { return false };
        if self.drag.is_some() || canvas.is_panning() {
            return true;
        }
        canvas.cancel_pan();
        let world = canvas.camera.screen_to_world(screen);
        match mode {
            TestMode::Spawn => {
                self.drag = Some(TestDrag::Spawn {
                    anchor_world: world,
                    anchor_screen: screen,
                    cursor_world: world,
                    dragged: false,
                    square: shift,
                });
            }
            TestMode::Erase => {
                self.erase_at_cursor(canvas, registry);
            }
            TestMode::Select => {
                if let Some(index) = self.selected {
                    if let Some(original) = canvas.instances.get(index).copied() {
                        if let Some(corner) = self.handle_at(canvas, registry, screen) {
                            self.drag = Some(TestDrag::Resize {
                                index,
                                original,
                                corner,
                                grab_offset: world - corner.position(&original),
                                anchor_screen: screen,
                            });
                            return true;
                        }
                    }
                }
                self.selected = geometry::pick(canvas, world, registry);
                if let Some(index) = self.selected {
                    self.drag = Some(TestDrag::Move {
                        index,
                        original: canvas.instances[index],
                        anchor_world: world,
                        anchor_screen: screen,
                    });
                }
            }
        }
        true
    }

    pub fn pointer_moved(
        &mut self,
        canvas: &mut Canvas,
        registry: &ShapeRegistry,
        screen: Vec2,
        shift: bool,
    ) -> bool {
        self.cursor_screen = screen;
        let world = canvas.camera.screen_to_world(screen);
        let Some(drag) = self.drag else { return self.mode.is_some() };
        let (index, result) = match drag {
            TestDrag::Spawn { anchor_world, anchor_screen, dragged, .. } => {
                self.drag = Some(TestDrag::Spawn {
                    anchor_world,
                    anchor_screen,
                    cursor_world: world,
                    dragged: dragged || screen.distance(anchor_screen) >= DRAG_THRESHOLD_PX,
                    square: shift,
                });
                return true;
            }
            TestDrag::Move { index, original, anchor_world, anchor_screen } => {
                let mut result = original;
                if screen != anchor_screen {
                    result.center = (Vec2::from_array(original.center) + (world - anchor_world)).to_array();
                }
                (index, result)
            }
            TestDrag::Resize { index, original, corner, grab_offset, anchor_screen } => (
                index,
                if screen == anchor_screen {
                    original
                } else {
                    geometry::resize_from_corner(original, corner, world - grab_offset, shift, registry)
                },
            ),
        };
        if let Some(shape) = canvas.instances.get_mut(index) {
            *shape = result;
        } else {
            self.drag = None;
            self.selected = None;
        }
        true
    }

    pub fn finish_interaction(&mut self, canvas: &mut Canvas) {
        let Some(drag) = self.drag.take() else { return };
        match drag {
            TestDrag::Spawn { dragged, .. } => {
                if let Some(shape) = self.spawn_preview(canvas, Some(drag)) {
                    if dragged {
                        self.set_spawn_size(Vec2::from_array(shape.size));
                    }
                    self.insert_spawn(canvas, shape);
                }
            }
            TestDrag::Move { index, original, .. }
            | TestDrag::Resize { index, original, .. } => {
                if let Some(result) = canvas.instances.get(index).copied() {
                    if result != original {
                        canvas.instances[index] = original;
                        canvas.record_history();
                        canvas.instances[index] = result;
                    }
                }
            }
        }
    }

    pub fn cancel_interaction(&mut self, canvas: &mut Canvas) -> bool {
        let Some(drag) = self.drag.take() else { return false };
        match drag {
            TestDrag::Move { index, original, .. }
            | TestDrag::Resize { index, original, .. } => {
                if let Some(shape) = canvas.instances.get_mut(index) {
                    *shape = original;
                }
            }
            TestDrag::Spawn { .. } => {}
        }
        true
    }

    pub fn escape(&mut self, canvas: &mut Canvas) {
        if !self.cancel_interaction(canvas) {
            self.clear_selection();
        }
    }

    pub fn spawn(&mut self, canvas: &mut Canvas) {
        if self.mode != Some(TestMode::Spawn) {
            return;
        }
        if self.is_dragging() {
            self.finish_interaction(canvas);
            return;
        }
        if let Some(shape) = self.preview_instance(canvas) {
            self.insert_spawn(canvas, shape);
        }
    }

    fn insert_spawn(&mut self, canvas: &mut Canvas, shape: ShapeInstance) {
        canvas.add_shape(shape);
        self.palette_cursor = self.palette_cursor.wrapping_add(1);
        self.selected = None;
    }

    pub fn erase_at_cursor(&mut self, canvas: &mut Canvas, registry: &ShapeRegistry) -> bool {
        if self.mode != Some(TestMode::Erase) {
            return false;
        }
        let world = canvas.camera.screen_to_world(self.cursor_screen);
        let Some(index) = geometry::pick(canvas, world, registry) else { return false };
        self.clear_selection();
        canvas.erase_at(index)
    }

    pub fn delete_selected(&mut self, canvas: &mut Canvas) {
        if self.mode != Some(TestMode::Select) {
            return;
        }
        self.finish_interaction(canvas);
        if let Some(index) = self.selected.take() {
            canvas.erase_at(index);
        }
    }

    pub fn preview_instance(&self, canvas: &Canvas) -> Option<ShapeInstance> {
        self.spawn_preview(canvas, self.drag)
    }

    fn spawn_preview(&self, canvas: &Canvas, drag: Option<TestDrag>) -> Option<ShapeInstance> {
        if self.mode != Some(TestMode::Spawn) {
            return None;
        }
        let spec = self.current?;
        let color = self.preview_color()?;
        if let Some(TestDrag::Spawn { anchor_world, cursor_world, dragged: true, square, .. }) = drag {
            let delta = cursor_world - anchor_world;
            let mut requested = delta.abs();
            if square {
                requested = Vec2::splat(requested.max_element());
            }
            let mut shape = geometry::build_instance(spec, anchor_world, requested, color);
            let direction = Vec2::new(
                if delta.x < 0.0 { -1.0 } else { 1.0 },
                if delta.y < 0.0 { -1.0 } else { 1.0 },
            );
            shape.center = (anchor_world + direction * Vec2::from_array(shape.size) * 0.5).to_array();
            return Some(shape);
        }
        let center = if let Some(TestDrag::Spawn { anchor_world, .. }) = drag {
            anchor_world
        } else {
            canvas.camera.screen_to_world(self.cursor_screen)
        };
        Some(geometry::build_instance(spec, center, self.spawn_size, color))
    }

    pub(super) fn handle_at(
        &self,
        canvas: &Canvas,
        registry: &ShapeRegistry,
        screen: Vec2,
    ) -> Option<Corner> {
        let shape = canvas.instances.get(self.selected?)?;
        if !geometry::can_resize(shape, registry) {
            return None;
        }
        CORNERS.into_iter()
            .map(|corner| {
                let delta = canvas.camera.world_to_screen(corner.position(shape)) - screen;
                (corner, delta)
            })
            .filter(|(_, delta)| delta.abs().max_element() <= HANDLE_HIT_PX)
            .min_by(|(_, a), (_, b)| a.length_squared().total_cmp(&b.length_squared()))
            .map(|(corner, _)| corner)
    }

    pub fn next_picked_color(&mut self) {
        let index = TEST_PALETTE.iter().position(|&color| color == self.picked_color)
            .map_or(0, |i| (i + 1) % TEST_PALETTE.len());
        self.picked_color = TEST_PALETTE[index];
    }

    pub fn toggle_color_mode(&mut self) {
        self.color_mode = self.color_mode.cycle();
    }

    pub fn preview_color(&self) -> Option<Color> {
        if self.mode != Some(TestMode::Spawn) {
            return None;
        }
        let spec = self.current?;
        Some(match self.color_mode {
            ColorMode::Cycle => TEST_PALETTE[self.palette_cursor % TEST_PALETTE.len()],
            ColorMode::Fixed => fixed_color(spec),
            ColorMode::Picked => self.picked_color,
        })
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
    fn cycle(self) -> Self {
        match self {
            Self::Cycle => Self::Fixed,
            Self::Fixed => Self::Picked,
            Self::Picked => Self::Cycle,
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
