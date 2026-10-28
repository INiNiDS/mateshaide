use super::{geometry, TestMode, overlay};
use super::spawner::{TestShapeSpec, TestSpawner};
use crate::{Canvas, Color, ShapeInstance, ShapeRegistry, palette};
use glam::Vec2;
use crate::dev::geometry::CORNERS;

fn setup(mode: TestMode) -> (Canvas, ShapeRegistry, TestSpawner) {
    let registry = ShapeRegistry::with_builtins().unwrap();
    let mut tool = TestSpawner::default();
    tool.select("rect", registry.kind("rect").unwrap());
    tool.toggle_mode(mode);
    (Canvas::new(800.0, 600.0), registry, tool)
}

fn rect() -> ShapeInstance {
    ShapeInstance::rect(-40.0, -30.0, 80.0, 60.0, Color::RED)
}

fn screen(canvas: &Canvas, world: Vec2) -> Vec2 {
    canvas.camera.world_to_screen(world)
}

fn close(a: Vec2, b: Vec2) {
    assert!((a - b).abs().max_element() < 0.002, "{a:?} != {b:?}");
}

#[test]
fn default_size_is_not_zero() {
    let tool = TestSpawner::default();
    assert_eq!(tool.spawn_size, Vec2::new(80.0, 60.0));
}

#[test]
fn explicit_spawn_size_updates_quad_and_sdf() {
    let (mut canvas, _, mut tool) = setup(TestMode::Spawn);
    tool.set_spawn_size(Vec2::new(240.0, 120.0));
    tool.spawn(&mut canvas);
    assert_eq!(canvas.instances[0].size, [240.0, 120.0]);
    assert_eq!(canvas.instances[0].a, [120.0, 60.0, 0.0, 0.0]);
    assert!(canvas.undo());
    assert!(canvas.instances.is_empty());
}

#[test]
fn circle_fits_requested_box_and_stays_round() {
    let registry = ShapeRegistry::with_builtins().unwrap();
    let spec = TestShapeSpec::new("circle", registry.kind("circle").unwrap());
    let shape = geometry::build_instance(spec, Vec2::ZERO, Vec2::new(140.0, 60.0), Color::RED);
    assert_eq!(shape.size, [60.0, 60.0]);
    assert_eq!(shape.a[0], 30.0);
}

#[test]
fn hexagon_uses_apothem_and_tight_unclipped_bounds() {
    let registry = ShapeRegistry::with_builtins().unwrap();
    let spec = TestShapeSpec::new("hexagon", registry.kind("hexagon").unwrap());
    let shape = geometry::build_instance(spec, Vec2::ZERO, Vec2::splat(80.0), Color::RED);
    close(
        Vec2::from_array(shape.size),
        Vec2::new(80.0, 80.0 * 0.866_025_4),
    );
    assert!((shape.a[0] * 2.0 - shape.size[1]).abs() < 0.001);
    assert!(geometry::contains(&shape, Vec2::ZERO, &registry));
    assert!(!geometry::contains(
        &shape,
        Vec2::new(39.0, 33.0),
        &registry
    ));
}

#[test]
fn rounded_rect_params_describe_outer_bounds() {
    let registry = ShapeRegistry::with_builtins().unwrap();
    let spec = TestShapeSpec::new("rounded_rect", registry.kind("rounded_rect").unwrap());
    let shape = geometry::build_instance(spec, Vec2::ZERO, Vec2::new(80.0, 60.0), Color::RED);
    assert_eq!(shape.a, [28.0, 18.0, 12.0, 0.0]);
    assert_eq!(shape.a[0] + shape.a[2], shape.size[0] * 0.5);
    assert!(!geometry::contains(
        &shape,
        Vec2::new(39.0, 29.0),
        &registry
    ));
    assert!(geometry::contains(&shape, Vec2::new(39.0, 0.0), &registry));
}

#[test]
fn picking_uses_visible_geometry_and_reverse_draw_order() {
    let (mut canvas, registry, _) = setup(TestMode::Select);
    canvas.instances.push(rect());
    canvas
        .instances
        .push(ShapeInstance::circle(0.0, 0.0, 20.0, Color::BLUE));
    assert_eq!(geometry::pick(&mut canvas, Vec2::ZERO, &registry), Some(1));
    assert_eq!(
        geometry::pick(&mut canvas, Vec2::new(19.0, 19.0), &registry),
        Some(0)
    );
    assert_eq!(
        geometry::pick(&mut canvas, Vec2::splat(100.0), &registry),
        None
    );
}

#[test]
fn line_hit_test_reads_second_endpoint_from_a() {
    let shape = ShapeInstance::line(Vec2::ZERO, Vec2::new(100.0, 0.0), 2.0, Color::RED);
    assert!(shape.contains_world(Vec2::new(90.0, 0.0)));
    assert!(!shape.contains_world(Vec2::new(90.0, 10.0)));
}

#[test]
fn selection_click_has_no_history_entry() {
    let (mut canvas, registry, mut tool) = setup(TestMode::Select);
    canvas.instances.push(rect());
    let cursor = screen(&canvas, Vec2::ZERO);
    tool.pointer_pressed(&mut canvas, &registry, cursor, false);
    tool.pointer_moved(&mut canvas, &registry, cursor, false);
    tool.finish_interaction(&mut canvas);
    assert_eq!(tool.selected(), Some(0));
    assert!(!canvas.undo());
}

#[test]
fn one_move_with_many_events_is_one_undo_step() {
    let (mut canvas, registry, mut tool) = setup(TestMode::Select);
    let original = rect();
    canvas.instances.push(original);
    let start = screen(&canvas, Vec2::ZERO);
    tool.pointer_pressed(&mut canvas, &registry, start, false);
    for i in 1..=20 {
        let cursor = screen(&canvas, Vec2::new(i as f32, i as f32 * 2.0));
        tool.pointer_moved(&mut canvas, &registry, cursor, false);
    }
    tool.finish_interaction(&mut canvas);
    let moved = canvas.instances[0];
    assert_eq!(moved.center, [20.0, 40.0]);
    assert!(canvas.undo());
    assert_eq!(canvas.instances[0], original);
    assert!(!canvas.undo());
    assert!(canvas.redo());
    assert_eq!(canvas.instances[0], moved);
}

#[test]
fn cancelled_move_restores_geometry_and_preserves_redo() {
    let (mut canvas, registry, mut tool) = setup(TestMode::Select);
    canvas.instances.push(rect());
    canvas.add_shape(ShapeInstance::circle(200.0, 0.0, 20.0, Color::BLUE));
    assert!(canvas.undo());
    let start = screen(&canvas, Vec2::ZERO);
    tool.pointer_pressed(&mut canvas, &registry, start, false);
    tool.pointer_moved(&mut canvas, &registry, start + Vec2::new(10.0, 10.0), false);
    assert!(tool.cancel_interaction(&mut canvas));
    assert_eq!(canvas.instances[0], rect());
    assert!(canvas.redo());
    assert_eq!(canvas.instances.len(), 2);
}

#[test]
fn click_preserves_redo() {
    let (mut canvas, registry, mut tool) = setup(TestMode::Select);
    canvas.instances.push(rect());
    canvas.add_shape(ShapeInstance::circle(200.0, 0.0, 20.0, Color::BLUE));
    assert!(canvas.undo());
    let cursor = screen(&canvas, Vec2::ZERO);
    tool.pointer_pressed(&mut canvas, &registry, cursor, false);
    tool.finish_interaction(&mut canvas);
    assert!(canvas.redo());
}

#[test]
fn all_resize_corners_keep_the_opposite_corner_fixed() {
    let registry = ShapeRegistry::with_builtins().unwrap();
    let original = rect();
    for corner in CORNERS {
        let anchor =
            Vec2::from_array(original.center) - corner.0 * Vec2::from_array(original.size) * 0.5;
        let moving = corner.position(&original) + corner.0 * Vec2::new(40.0, 30.0);
        let result = geometry::resize_from_corner(original, corner, moving, false, &registry);
        close(Vec2::from_array(result.size), Vec2::new(120.0, 90.0));
        close(
            Vec2::from_array(result.center) - corner.0 * Vec2::from_array(result.size) * 0.5,
            anchor,
        );
        assert_eq!(result.a, [60.0, 45.0, 0.0, 0.0]);
    }
}

#[test]
fn shift_resize_preserves_original_aspect_ratio() {
    let registry = ShapeRegistry::with_builtins().unwrap();
    let original = rect();
    let corner = CORNERS[2];
    let result =
        geometry::resize_from_corner(original, corner, Vec2::new(100.0, 40.0), true, &registry);
    assert!((result.size[0] / result.size[1] - 80.0 / 60.0).abs() < 0.001);
}

#[test]
fn circle_resize_is_uniform_without_shift() {
    let registry = ShapeRegistry::with_builtins().unwrap();
    let original = ShapeInstance::circle(0.0, 0.0, 40.0, Color::RED);
    let result = geometry::resize_from_corner(
        original,
        CORNERS[2],
        Vec2::new(120.0, 50.0),
        false,
        &registry,
    );
    assert_eq!(result.size[0], result.size[1]);
    assert_eq!(result.a[0], result.size[0] * 0.5);
}

#[test]
fn corner_click_does_not_create_an_undo_entry() {
    let (mut canvas, registry, mut tool) = setup(TestMode::Select);
    canvas.instances.push(rect());
    let center = screen(&canvas, Vec2::ZERO);
    tool.pointer_pressed(&mut canvas, &registry, center, false);
    tool.finish_interaction(&mut canvas);
    let cursor = screen(&canvas, CORNERS[2].position(&canvas.instances[0]));
    tool.pointer_pressed(&mut canvas, &registry, cursor, false);
    tool.pointer_moved(&mut canvas, &registry, cursor, false);
    tool.finish_interaction(&mut canvas);
    assert!(!canvas.undo());
}

#[test]
fn resize_drag_commits_one_history_entry() {
    let (mut canvas, registry, mut tool) = setup(TestMode::Select);
    canvas.instances.push(rect());
    let center = screen(&canvas, Vec2::ZERO);
    tool.pointer_pressed(&mut canvas, &registry, center, false);
    tool.finish_interaction(&mut canvas);
    let cursor = screen(&canvas, CORNERS[2].position(&canvas.instances[0]));
    tool.pointer_pressed(&mut canvas, &registry, cursor, false);
    tool.pointer_moved(
        &mut canvas,
        &registry,
        cursor + Vec2::new(40.0, -30.0),
        false,
    );
    tool.finish_interaction(&mut canvas);
    assert_eq!(canvas.instances[0].size, [120.0, 90.0]);
    assert!(canvas.undo());
    assert_eq!(canvas.instances[0], rect());
    assert!(!canvas.undo());
}

#[test]
fn spawn_drag_is_transient_then_remembers_size() {
    let (mut canvas, registry, mut tool) = setup(TestMode::Spawn);
    let start = screen(&canvas, Vec2::new(-100.0, -100.0));
    let end = screen(&canvas, Vec2::new(140.0, 20.0));
    tool.pointer_pressed(&mut canvas, &registry, start, false);
    tool.pointer_moved(&mut canvas, &registry, end, false);
    assert!(canvas.instances.is_empty());
    assert_eq!(tool.preview_instance(&canvas).unwrap().size, [240.0, 120.0]);
    tool.finish_interaction(&mut canvas);
    assert_eq!(tool.spawn_size, Vec2::new(240.0, 120.0));
    assert_eq!(canvas.instances[0].center, [20.0, -40.0]);
    tool.spawn(&mut canvas);
    assert_eq!(canvas.instances[1].size, [240.0, 120.0]);
}

#[test]
fn spawn_click_uses_configured_size_instead_of_minimum() {
    let (mut canvas, registry, mut tool) = setup(TestMode::Spawn);
    let cursor = screen(&canvas, Vec2::ZERO);
    tool.set_spawn_size(Vec2::new(150.0, 90.0));
    tool.pointer_pressed(&mut canvas, &registry, cursor, false);
    tool.finish_interaction(&mut canvas);
    assert_eq!(canvas.instances[0].size, [150.0, 90.0]);
    assert_eq!(canvas.instances[0].center, [0.0, 0.0]);
}

#[test]
fn cancelled_spawn_changes_neither_canvas_nor_palette() {
    let (mut canvas, registry, mut tool) = setup(TestMode::Spawn);
    let color = tool.preview_color();
    let cursor = screen(&canvas, Vec2::ZERO);
    tool.pointer_pressed(&mut canvas, &registry, cursor, false);
    tool.pointer_moved(&mut canvas, &registry, cursor + Vec2::splat(100.0), false);
    tool.escape(&mut canvas);
    assert!(canvas.instances.is_empty());
    assert_eq!(tool.preview_color(), color);
    assert!(!canvas.undo());
}

#[test]
fn palette_advances_once_and_preview_does_not_advance_it() {
    let (mut canvas, _, mut tool) = setup(TestMode::Spawn);
    assert_eq!(tool.preview_color(), Some(palette::RED_500));
    for _ in 0..20 {
        let _ = tool.preview_instance(&canvas);
    }
    tool.spawn(&mut canvas);
    assert_eq!(canvas.instances[0].color, palette::RED_500);
    assert_eq!(tool.preview_color(), Some(palette::GREEN_500));
    tool.spawn(&mut canvas);
    assert_eq!(canvas.instances[1].color, palette::GREEN_500);
}

#[test]
fn selection_handles_have_constant_screen_hit_radius() {
    let (mut canvas, registry, mut tool) = setup(TestMode::Select);
    canvas.instances.push(rect());
    let cursor = screen(&canvas, Vec2::ZERO);
    tool.pointer_pressed(&mut canvas, &registry, cursor, false);
    tool.finish_interaction(&mut canvas);
    for zoom in [0.1, 1.0, 10.0] {
        canvas.camera.zoom = zoom;
        let corner = screen(&canvas, CORNERS[2].position(&canvas.instances[0]));
        assert!(
            tool.handle_at(&canvas, &registry, corner + Vec2::new(6.0, 0.0))
                .is_some()
        );
        if zoom >= 1.0 {
            assert!(
                tool.handle_at(&canvas, &registry, corner + Vec2::new(8.0, 0.0))
                    .is_none()
            );
        }
    }
}

#[test]
fn overlays_do_not_modify_canvas_or_history() {
    let (mut canvas, registry, mut tool) = setup(TestMode::Select);
    canvas.instances.push(rect());
    let cursor = screen(&canvas, Vec2::ZERO);
    tool.pointer_pressed(&mut canvas, &registry, cursor, false);
    tool.finish_interaction(&mut canvas);
    let before = canvas.instances.clone();
    let instances = overlay::build_instances(&canvas, &registry, &tool);
    assert_eq!(instances.len(), 12);
    assert_eq!(canvas.instances, before);
    assert!(!canvas.undo());
}

#[test]
fn delete_selection_is_undoable() {
    let (mut canvas, registry, mut tool) = setup(TestMode::Select);
    canvas.instances.push(rect());
    let cursor = screen(&canvas, Vec2::ZERO);
    tool.pointer_pressed(&mut canvas, &registry, cursor, false);
    tool.finish_interaction(&mut canvas);
    tool.delete_selected(&mut canvas);
    assert!(canvas.instances.is_empty());
    assert_eq!(tool.selected(), None);
    assert!(canvas.undo());
    assert_eq!(canvas.instances[0], rect());
}

#[test]
fn size_validation_rejects_nan_and_clamps_limits() {
    let mut tool = TestSpawner::default();
    let before = tool.spawn_size;
    tool.set_spawn_size(Vec2::new(f32::NAN, 100.0));
    assert_eq!(tool.spawn_size, before);
    tool.set_spawn_size(Vec2::new(-10.0, 100_000.0));
    assert_eq!(
        tool.spawn_size,
        Vec2::new(geometry::MIN_SIZE, geometry::MAX_SIZE)
    );
    tool.scale_spawn_size(-1.0);
    assert_eq!(
        tool.spawn_size,
        Vec2::new(geometry::MIN_SIZE, geometry::MAX_SIZE)
    );
}

#[test]
fn unknown_sdf_and_lines_are_not_given_invalid_resize_parameters() {
    let registry = ShapeRegistry::with_builtins().unwrap();
    let line = ShapeInstance::line(Vec2::ZERO, Vec2::splat(20.0), 2.0, Color::RED);
    let unknown = ShapeInstance::new(99, Vec2::ZERO, Vec2::splat(50.0), Color::RED);
    assert!(!geometry::can_resize(&line, &registry));
    assert!(!geometry::can_resize(&unknown, &registry));
}

#[test]
fn move_uses_world_delta_at_non_default_zoom_and_pan() {
    let (mut canvas, registry, mut tool) = setup(TestMode::Select);
    canvas.instances.push(rect());
    canvas.camera.zoom = 2.5;
    canvas.camera.offset = Vec2::new(100.0, -200.0);
    let cursor = screen(&canvas, Vec2::new(20.0, -10.0));
    tool.pointer_pressed(&mut canvas, &registry, cursor, false);
    tool.pointer_moved(
        &mut canvas,
        &registry,
        cursor + Vec2::new(25.0, -50.0),
        false,
    );
    tool.finish_interaction(&mut canvas);
    close(
        Vec2::from_array(canvas.instances[0].center),
        Vec2::new(10.0, 20.0),
    );
}

#[test]
fn hexagon_resize_preserves_regular_geometry_without_shift() {
    let registry = ShapeRegistry::with_builtins().unwrap();
    let spec = TestShapeSpec::new("hexagon", registry.kind("hexagon").unwrap());
    let original = geometry::build_instance(spec, Vec2::ZERO, Vec2::splat(80.0), Color::RED);
    let result = geometry::resize_from_corner(
        original,
        CORNERS[2],
        Vec2::new(100.0, 60.0),
        false,
        &registry,
    );
    assert!((result.size[1] - result.size[0] * 0.866_025_4).abs() < 0.002);
    assert_eq!(result.a[0], result.size[1] * 0.5);
}

#[test]
fn rounded_resize_updates_inner_box_and_corner_radius() {
    let registry = ShapeRegistry::with_builtins().unwrap();
    let spec = TestShapeSpec::new("rounded_rect", registry.kind("rounded_rect").unwrap());
    let original = geometry::build_instance(spec, Vec2::ZERO, Vec2::new(80.0, 60.0), Color::RED);
    let result = geometry::resize_from_corner(
        original,
        CORNERS[2],
        Vec2::new(80.0, 60.0),
        false,
        &registry,
    );
    assert_eq!(result.size, [120.0, 90.0]);
    assert_eq!(result.a, [42.0, 27.0, 18.0, 0.0]);
}

#[test]
fn moving_back_to_the_press_position_is_a_no_op() {
    let (mut canvas, registry, mut tool) = setup(TestMode::Select);
    canvas.instances.push(rect());
    let cursor = screen(&canvas, Vec2::ZERO);
    tool.pointer_pressed(&mut canvas, &registry, cursor, false);
    tool.pointer_moved(
        &mut canvas,
        &registry,
        cursor + Vec2::new(40.0, 10.0),
        false,
    );
    tool.pointer_moved(&mut canvas, &registry, cursor, false);
    tool.finish_interaction(&mut canvas);
    assert_eq!(canvas.instances[0], rect());
    assert!(!canvas.undo());
}
