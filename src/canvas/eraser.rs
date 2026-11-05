use super::hit::{self, Hit};
use super::model::Canvas;
use crate::shape::ShapeRegistry;
use glam::Vec2;

pub const ERASER_RADIUS_PX: f32 = 6.0;

#[derive(Debug, Default)]
pub struct Eraser {
    pub enabled: bool,
    pub hover: Option<Hit>,
    dragging: bool,
    batch: bool,
    erased: usize,
    last: Option<Vec2>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct EraseOutcome {
    pub redraw: bool,
    pub nodes: bool,
}

impl EraseOutcome {
    fn merge(&mut self, other: Self) {
        self.redraw |= other.redraw;
        self.nodes |= other.nodes;
    }
}

impl Eraser {
    #[must_use]
    pub fn is_dragging(&self) -> bool {
        self.dragging
    }

    pub fn set_enabled(&mut self, canvas: &mut Canvas, enabled: bool) {
        if !enabled {
            self.release(canvas);
            self.hover = None;
        }
        self.enabled = enabled;
    }

    fn tolerance(canvas: &Canvas) -> f32 {
        ERASER_RADIUS_PX / canvas.camera.zoom.max(1e-3)
    }

    pub fn press(
        &mut self,
        canvas: &mut Canvas,
        registry: &ShapeRegistry,
        screen: Vec2,
    ) -> EraseOutcome {
        if !self.enabled {
            return EraseOutcome::default();
        }
        self.release(canvas);
        let world = canvas.camera.screen_to_world(screen);
        self.batch = canvas.begin_batch();
        self.dragging = true;
        self.erased = 0;
        self.last = Some(world);
        let mut out = self.erase_point(canvas, registry, world);
        out.merge(self.update_hover(canvas, registry, world));
        out
    }

    pub fn moved(
        &mut self,
        canvas: &mut Canvas,
        registry: &ShapeRegistry,
        screen: Vec2,
    ) -> EraseOutcome {
        if !self.enabled {
            return EraseOutcome::default();
        }
        let world = canvas.camera.screen_to_world(screen);
        let mut out = EraseOutcome::default();
        if self.dragging {
            let from = self.last.unwrap_or(world);
            let step = Self::tolerance(canvas).max(0.5);
            let steps = ((world - from).length() / step).ceil().clamp(1.0, 256.0) as usize;
            for i in 1..=steps {
                let p = from.lerp(world, i as f32 / steps as f32);
                out.merge(self.erase_point(canvas, registry, p));
            }
            self.last = Some(world);
        }
        out.merge(self.update_hover(canvas, registry, world));
        out
    }

    pub fn release(&mut self, canvas: &mut Canvas) {
        if !self.dragging {
            return;
        }
        self.dragging = false;
        self.last = None;
        if self.erased == 0 && self.batch {
            canvas.discard_batch();
        }
        self.batch = false;
        self.erased = 0;
    }

    fn erase_point(
        &mut self,
        canvas: &mut Canvas,
        registry: &ShapeRegistry,
        world: Vec2,
    ) -> EraseOutcome {
        let Some(hit) = hit::hit_at(canvas, world, registry, Self::tolerance(canvas)) else {
            return EraseOutcome::default();
        };
        if !canvas.remove_hit(hit) {
            return EraseOutcome::default();
        }
        self.erased += 1;
        self.hover = None;
        EraseOutcome {
            redraw: true,
            nodes: matches!(hit, Hit::Node(_)),
        }
    }

    fn update_hover(
        &mut self,
        canvas: &Canvas,
        registry: &ShapeRegistry,
        world: Vec2,
    ) -> EraseOutcome {
        let hover = hit::hit_at(canvas, world, registry, Self::tolerance(canvas));
        let changed = hover != self.hover;
        self.hover = hover;
        EraseOutcome {
            redraw: changed,
            nodes: false,
        }
    }
}
