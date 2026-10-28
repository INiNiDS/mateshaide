use super::camera::Camera;
use super::history::History;
use super::text::CanvasText;
use crate::core::Color;
use crate::shape::ShapeInstance;
use glam::Vec2;
use std::collections::VecDeque;
use web_time::Instant;

const MIN_STROKE_STEP: f32 = 2.0;
const STEP_PIXELS: f32 = 50.0;
const FRICTION: f32 = 12.0;
const VELOCITY_SAMPLE_WINDOW_SECS: f32 = 0.08;

pub struct Canvas {
    pub camera: Camera,
    pub instances: Vec<ShapeInstance>,
    pan_velocity: Vec2,
    pan_samples: VecDeque<(Instant, Vec2)>,
    is_panning: bool,
    history: History,
    stroke_active: bool,
    stroke_last: Option<Vec2>,
    stroke_segments: usize,
    pub texts: Vec<CanvasText>,
}

impl Canvas {
    #[must_use]
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            camera: Camera::new(width, height),
            pan_velocity: Vec2::ZERO,
            instances: Vec::new(),
            pan_samples: VecDeque::with_capacity(64),
            is_panning: false,
            history: History::new(),
            stroke_active: false,
            stroke_last: None,
            stroke_segments: 0,
            texts: Vec::new(),
        }
    }

    #[cfg(debug_assertions)]
    pub(crate) fn record_history(&mut self) {
        self.history.record_snapshot(&self.instances);
    }

    pub fn add_shape(&mut self, shape: ShapeInstance) {
        self.history.record_snapshot(&self.instances);
        self.instances.push(shape);
    }

    pub fn add_text(&mut self, text: CanvasText) {
        self.history.record_snapshot(&self.instances);
        self.texts.push(text);
    }

    pub fn erase_at(&mut self, index: usize) -> bool {
        if index >= self.instances.len() {
            return false;
        }

        self.history.record_snapshot(&self.instances);
        self.instances.remove(index);
        true
    }

    pub fn erase_at_world(&mut self, world: Vec2) -> bool {
        let idx = self
            .instances
            .iter()
            .enumerate()
            .rev()
            .find(|(_, shape)| shape.contains_world(world))
            .map(|(i, _)| i);

        match idx {
            Some(i) => {
                self.erase_at(i);
                true
            }
            None => false,
        }
    }

    pub fn undo(&mut self) -> bool {
        self.history.undo(&mut self.instances)
    }

    pub fn redo(&mut self) -> bool {
        self.history.redo(&mut self.instances)
    }

    #[must_use]
    pub fn is_stroking(&self) -> bool {
        self.stroke_active
    }

    pub fn begin_stroke(&mut self) {
        if self.stroke_active {
            return;
        }

        self.history.record_snapshot(&self.instances);
        self.stroke_active = true;
        self.stroke_last = None;
        self.stroke_segments = 0;
    }

    pub fn extend_stroke(&mut self, world: Vec2, width: f32, color: Color) {
        if !self.stroke_active {
            return;
        }

        match self.stroke_last {
            None => self.stroke_last = Some(world),
            Some(last) => {
                if last.distance(world) < MIN_STROKE_STEP {
                    return;
                }

                self.instances
                    .push(ShapeInstance::line(last, world, width * 0.5, color));

                self.stroke_last = Some(world);
                self.stroke_segments += 1;
            }
        }
    }

    pub fn end_stroke(&mut self) {
        if !self.stroke_active {
            return;
        }

        self.stroke_active = false;
        self.stroke_last = None;

        if self.stroke_segments == 0 {
            self.history.discard_last_undo();
        }
    }

    pub fn resize(&mut self, width: f32, height: f32) {
        self.camera.resize(width, height);
    }

    pub fn start_pan(&mut self) {
        self.is_panning = true;
        self.pan_velocity = Vec2::ZERO;
        self.pan_samples.clear();
    }

    #[must_use]
    pub fn is_panning(&self) -> bool {
        self.is_panning
    }

    pub fn track_pan(&mut self, delta: Vec2) {
        if !self.is_panning {
            return;
        }

        let now = Instant::now();
        self.pan_samples.push_back((now, delta));

        self.prune_pan_samples(now);
        self.pan(delta);
    }

    pub fn end_pan(&mut self) {
        if !self.is_panning {
            return;
        }

        let now = Instant::now();

        self.is_panning = false;
        self.prune_pan_samples(now);

        if let (Some(&(oldest_time, _)), Some(&(newest_time, _))) =
            (self.pan_samples.front(), self.pan_samples.back())
        {
            let duration = (newest_time - oldest_time).as_secs_f32();
            let idle_time = (now - newest_time).as_secs_f32();

            if duration > 0.01 && idle_time < 0.04 {
                let total_delta: Vec2 = self.pan_samples.iter().map(|(_, delta)| *delta).sum();

                self.pan_velocity = total_delta / duration;
            } else {
                self.pan_velocity = Vec2::ZERO;
            }
        } else {
            self.pan_velocity = Vec2::ZERO;
        }

        self.pan_samples.clear();
    }

    pub fn pan(&mut self, delta: Vec2) {
        self.camera.offset.x -= delta.x / self.camera.zoom;
        self.camera.offset.y += delta.y / self.camera.zoom;
    }

    // Force cancel pan
    pub fn cancel_pan(&mut self) {
        self.is_panning = false;
        self.pan_velocity = Vec2::ZERO;
        self.pan_samples.clear();
    }

    pub fn pan_step(&mut self, direction: Vec2) {
        self.pan(direction * STEP_PIXELS);
    }

    pub fn zoom(&mut self, cursor_pos: Vec2, delta: f32) {
        self.camera.zoom_at(cursor_pos, delta);
    }

    pub fn zoom_at_center(&mut self, delta: f32) {
        let center = self.camera.screen_size * 0.5;
        self.camera.zoom_at(center, delta);
    }

    pub fn reset_view(&mut self) {
        self.camera.zoom = 1.0;
        self.camera.offset = Vec2::ZERO;
        self.pan_velocity = Vec2::ZERO;
    }

    pub fn update(&mut self, delta_time: f32) -> bool {
        if !self.is_panning && self.pan_velocity.length_squared() > 1.0 {
            self.pan(self.pan_velocity * delta_time);
            self.pan_velocity *= (-FRICTION * delta_time).exp();

            if self.pan_velocity.length_squared() < 4.0 {
                self.pan_velocity = Vec2::ZERO;
            }

            return true;
        }

        false
    }

    fn prune_pan_samples(&mut self, now: Instant) {
        while let Some(&(time, _)) = self.pan_samples.front() {
            if (now - time).as_secs_f32() > VELOCITY_SAMPLE_WINDOW_SECS {
                self.pan_samples.pop_front();
            } else {
                break;
            }
        }
    }
}
