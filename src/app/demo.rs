use crate::core::Color;
use crate::engine::Engine;
use crate::palette;
use crate::shape::ShapeInstance;
use crate::canvas::CanvasText;
use glam::Vec2;

const HELP_TEXT: &str = "\
MateshaIDE: controls

DRAWING
  Ctrl+9              toggle draw mode
  LMB + drag          draw (when draw mode is on)

HISTORY
  Ctrl+Z              undo
  Ctrl+Y / Ctrl+X     redo
  Ctrl+Shift+Z        redo

CAMERA
  Mouse wheel         pan
  Shift + wheel       horizontal pan
  Ctrl + wheel        zoom at cursor
  RMB / MMB + drag    pan (with inertia)
  Ctrl+= / Ctrl+NumPad+    zoom in
  Ctrl+- / Ctrl+NumPad-    zoom out
  Ctrl+0 / Ctrl+NumPad0    reset view

EXPORT
  Ctrl+E              export to PNG (preview)
    S: save, C: copy, Esc: close
  Ctrl+P              export to PDF

DEV TOOLS (debug builds only)
  Ctrl+1              toggle shape spawn mode
  1 / LMB click       spawn at cursor with configured size
  LMB + drag          spawn with dragged size; remember that size
  [ / ]               decrease / increase spawn size
  Shift + [ / ]       change spawn width
  Alt + [ / ]         change spawn height
  Ctrl+2..5           rect / circle / hexagon / rounded_rect
  Ctrl+6              color mode: Cycle / Fixed / Picked
  Ctrl+7              next palette color (Picked)
  Ctrl+8              toggle eraser; LMB / 1 removes a shape
  V / Ctrl+Shift+1    toggle selection mode
  LMB + drag          select and move topmost shape
  Drag corner         resize selected shape
  Shift + resize      preserve aspect ratio
  Delete / Backspace  delete selected shape
  Esc                 cancel drag, otherwise deselect
  Circles / hexagons keep their proportions.
  Lines / custom SDF shapes can be moved, but not resized.
";

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

   engine.canvas.add_text(CanvasText {
        position: Vec2::new(-600.0, 400.0),
        text: HELP_TEXT.to_owned(),
        size: 16.0,
        color: palette::GRAY_700,
    });
}
