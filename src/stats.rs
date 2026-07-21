//! Stats panel data. Mirrors three.js's `Stats` (FPS + frame time + memory
//! ring buffers). DOM rendering is left to the integrator; this struct just
//! tracks the running averages.

use std::time::Duration;

const HISTORY: usize = 60;

pub struct Stats {
    fps_history: [f32; HISTORY],
    frame_ms_history: [f32; HISTORY],
    head: usize,
    last_t: Option<web_time::Instant>,
    pub fps: f32,
    pub frame_ms: f32,
    pub memory_mb: f32,
}

impl Default for Stats {
    fn default() -> Self {
        Self {
            fps_history: [0.0; HISTORY],
            frame_ms_history: [0.0; HISTORY],
            head: 0,
            last_t: None,
            fps: 0.0,
            frame_ms: 0.0,
            memory_mb: 0.0,
        }
    }
}

impl Stats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn begin(&mut self) {
        self.last_t = Some(web_time::Instant::now());
    }

    pub fn end(&mut self) {
        let Some(t0) = self.last_t else {
            return;
        };
        let dt = web_time::Instant::now().duration_since(t0);
        self.tick(dt);
    }

    pub fn tick(&mut self, delta: Duration) {
        let ms = delta.as_secs_f32() * 1000.0;
        self.frame_ms = ms;
        self.fps = if ms > 0.0 { 1000.0 / ms } else { 0.0 };
        self.fps_history[self.head] = self.fps;
        self.frame_ms_history[self.head] = ms;
        self.head = (self.head + 1) % HISTORY;
    }

    pub fn avg_fps(&self) -> f32 {
        let sum: f32 = self.fps_history.iter().sum();
        sum / HISTORY as f32
    }

    pub fn avg_frame_ms(&self) -> f32 {
        let sum: f32 = self.frame_ms_history.iter().sum();
        sum / HISTORY as f32
    }
}
