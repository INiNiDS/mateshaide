use super::hit::{self, Hit};
use super::model::Canvas;
use crate::plugins::PluginNode;
use crate::shape::ShapeRegistry;
use glam::Vec2;

pub const ERASER_RADIUS_PX: f32 = 6.0;
const CLICK_SLOP_PX: f32 = 4.0;
const MAX_SWEEP_STEPS: f32 = 256.0;

#[derive(Debug, Clone, PartialEq)]
pub struct InkErase {
    pub plugin: String,
    pub node: String,
    pub world: Vec2,
    pub click: bool,
    pub tolerance: f32,
}

#[derive(Debug, Default)]
pub struct Eraser {
    pub enabled: bool,
    pub hover: Option<Hit>,
    dragging: bool,
    batch: bool,
    erased: usize,
    last: Option<Vec2>,
    press_screen: Vec2,
    press_world: Vec2,
    press_hit: Option<Hit>,
    press_ink: bool,
    moved: bool,
    ink: Vec<InkErase>,
}

#[derive(Debug, Default, Clone, Copy)]
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

type Accepts<'a> = &'a dyn Fn(&PluginNode) -> bool;

impl Eraser {
    #[must_use]
    pub fn is_dragging(&self) -> bool {
        self.dragging
    }

    pub fn take_ink(&mut self) -> Vec<InkErase> {
        std::mem::take(&mut self.ink)
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
        accepts: Accepts<'_>,
    ) -> EraseOutcome {
        if !self.enabled {
            return EraseOutcome::default();
        }

        tracing::info!("ERASER press screen={screen:?}");
        let mut out = self.release(canvas);
        let world = canvas.camera.screen_to_world(screen);

        self.batch = canvas.begin_batch();
        self.dragging = true;
        self.moved = false;
        self.erased = 0;
        self.last = Some(world);
        self.press_screen = screen;
        self.press_world = world;
        self.press_hit = hit::hit_at(canvas, world, registry, Self::tolerance(canvas));
        self.press_ink = match self.press_hit {
            Some(Hit::Node(i)) => canvas.plugin_nodes.get(i).is_some_and(accepts),
            _ => false,
        };

        out.merge(self.update_hover(canvas, registry, world));
        out
    }

    pub fn moved(
        &mut self,
        canvas: &mut Canvas,
        registry: &ShapeRegistry,
        screen: Vec2,
        accepts: Accepts<'_>,
    ) -> EraseOutcome {
        if !self.enabled {
            return EraseOutcome::default();
        }

        let world = canvas.camera.screen_to_world(screen);
        let mut out = EraseOutcome::default();

        if self.dragging {
            tracing::info!(
                "ERASER drag moved={} dist={}",
                self.moved,
                (screen - self.press_screen).length()
            );
            let mut ink_at: Option<(Vec2, usize)> = None;

            if !self.moved && (screen - self.press_screen).length() > CLICK_SLOP_PX {
                self.moved = true;
                let start = self.press_world;
                out.merge(self.sweep(canvas, registry, start, accepts, &mut ink_at));
            }

            if self.moved {
                let from = self.last.unwrap_or(world);
                let step = Self::tolerance(canvas).max(0.5);
                let steps = ((world - from).length() / step)
                    .ceil()
                    .clamp(1.0, MAX_SWEEP_STEPS) as usize;

                for i in 1..=steps {
                    let p = from.lerp(world, i as f32 / steps as f32);
                    out.merge(self.sweep(canvas, registry, p, accepts, &mut ink_at));
                }
            }

            if let Some((p, i)) = ink_at {
                self.push_ink(canvas, i, p, false);
            }
            self.last = Some(world);
        }

        out.merge(self.update_hover(canvas, registry, world));
        out
    }

    pub fn release(&mut self, canvas: &mut Canvas) -> EraseOutcome {
        if !self.dragging {
            return EraseOutcome::default();
        }

        let mut out = EraseOutcome::default();
        if !self.moved
            && let Some(hit) = self.press_hit {
                out = self.erase_click(canvas, hit);
            }

        self.dragging = false;
        self.press_hit = None;
        self.last = None;

        if self.erased == 0 && self.batch {
            canvas.discard_batch();
        }
        self.batch = false;
        self.erased = 0;
        out
    }

    fn sweep(
        &mut self,
        canvas: &mut Canvas,
        registry: &ShapeRegistry,
        world: Vec2,
        accepts: Accepts<'_>,
        ink_at: &mut Option<(Vec2, usize)>,
    ) -> EraseOutcome {
        let tol = Self::tolerance(canvas);
        let under = canvas
            .instances
            .iter()
            .rposition(|s| hit::shape_hit(s, world, registry, tol))
            .map(|i| canvas.instances[i].stroke_id());
        tracing::info!(
            "ERASER sweep world={world:?} stroke={:?} shape_stroke_id={under:?}",
            stroke_at(canvas, registry, world, tol)
        );

        if let Some(group) = stroke_at(canvas, registry, world, tol) {
            let ok = canvas.remove_stroke(group);
            return self.removed(ok, false);
        }

        if let Some(i) = ink_node_at(canvas, world, accepts) {
            *ink_at = Some((world, i));
        }

        EraseOutcome::default()
    }

    fn erase_click(&mut self, canvas: &mut Canvas, hit: Hit) -> EraseOutcome {
        if let (true, Hit::Node(i)) = (self.press_ink, hit) {
            let world = self.press_world;
            self.push_ink(canvas, i, world, true);
            return EraseOutcome::default();
        }

        match hit {
            Hit::Shape(i) => {
                let group = canvas
                    .instances
                    .get(i)
                    .map_or(0, super::super::shape::instance::ShapeInstance::stroke_id);
                let ok = if group != 0 {
                    canvas.remove_stroke(group)
                } else {
                    canvas.remove_hit(hit)
                };
                self.removed(ok, false)
            }
            Hit::Node(_) => {
                let ok = canvas.remove_hit(hit);
                self.removed(ok, true)
            }
            Hit::Text(_) => {
                let ok = canvas.remove_hit(hit);
                self.removed(ok, false)
            }
        }
    }

    fn removed(&mut self, ok: bool, nodes: bool) -> EraseOutcome {
        if !ok {
            return EraseOutcome::default();
        }
        self.erased += 1;
        self.hover = None;
        EraseOutcome {
            redraw: true,
            nodes,
        }
    }

    fn push_ink(&mut self, canvas: &Canvas, index: usize, world: Vec2, click: bool) {
        if let Some(node) = canvas.plugin_nodes.get(index) {
            self.ink.push(InkErase {
                plugin: node.owner.clone(),
                node: node.id.clone(),
                world,
                click,
                tolerance: Self::tolerance(canvas),
            });
        }
    }

    fn update_hover(
        &mut self,
        canvas: &Canvas,
        registry: &ShapeRegistry,
        world: Vec2,
    ) -> EraseOutcome {
        self.hover = if self.enabled {
            hit::hit_at(canvas, world, registry, Self::tolerance(canvas))
        } else {
            None
        };
        EraseOutcome {
            redraw: self.enabled,
            nodes: false,
        }
    }
}

fn stroke_at(canvas: &Canvas, registry: &ShapeRegistry, world: Vec2, tol: f32) -> Option<u32> {
    canvas
        .instances
        .iter()
        .rev()
        .find(|s| s.stroke_id() != 0 && hit::shape_hit(s, world, registry, tol))
        .map(super::super::shape::instance::ShapeInstance::stroke_id)
}

fn ink_node_at(canvas: &Canvas, world: Vec2, accepts: Accepts<'_>) -> Option<usize> {
    canvas
        .plugin_nodes
        .iter()
        .enumerate()
        .rev()
        .find(|&(_, n)| accepts(n) && contains(n, world))
        .map(|(i, _)| i)
}

fn contains(n: &PluginNode, p: Vec2) -> bool {
    p.x >= n.x && p.x <= n.x + n.width && p.y <= n.y && p.y >= n.y - n.height
}
