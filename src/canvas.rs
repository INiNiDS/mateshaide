use crate::camera::Camera;
use glam::Vec2;
use std::collections::VecDeque;
use std::time::Instant;

const STEP_PIXELS: f32 = 50.0;
const FRICTION: f32 = 5.0;
const VELOCITY_SAMPLE_WINDOW_SECS: f32 = 0.08;

pub struct Canvas {
    pub camera: Camera,
    pan_velocity: Vec2,
    pan_samples: VecDeque<(Instant, Vec2)>,
    is_panning: bool,
}

impl Canvas {
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            camera: Camera::new(width, height),
            pan_velocity: Vec2::ZERO,
            pan_samples: VecDeque::with_capacity(64),
            is_panning: false,
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
                let total_delta: Vec2 = self.pan_samples.iter().map(|(_, d)| *d).sum();
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
