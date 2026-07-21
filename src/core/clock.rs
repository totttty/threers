use web_time::Instant;

/// Mirrors three.js's `Clock`. Wraps `Instant` so it works on native and wasm32.
#[derive(Debug, Clone, Copy)]
pub struct Clock {
    pub auto_start: bool,
    pub start_time: Instant,
    pub old_time: Instant,
    pub elapsed_time: f64,
    pub running: bool,
}

impl Default for Clock {
    fn default() -> Self {
        Self::new(true)
    }
}

impl Clock {
    pub fn new(auto_start: bool) -> Self {
        let now = Instant::now();
        Self {
            auto_start,
            start_time: now,
            old_time: now,
            elapsed_time: 0.0,
            running: false,
        }
    }

    pub fn start(&mut self) {
        let now = Instant::now();
        self.start_time = now;
        self.old_time = now;
        self.elapsed_time = 0.0;
        self.running = true;
    }

    pub fn stop(&mut self) {
        self.get_elapsed_time();
        self.running = false;
        self.auto_start = false;
    }

    /// Total seconds since `start()` was called (or auto-started on first read).
    pub fn get_elapsed_time(&mut self) -> f64 {
        self.get_delta();
        self.elapsed_time
    }

    /// Seconds since the previous call (or `start()`). Auto-starts the clock
    /// on the first call if `auto_start` is set.
    pub fn get_delta(&mut self) -> f64 {
        let mut delta = 0.0;
        if self.auto_start && !self.running {
            self.start();
            return 0.0;
        }
        if self.running {
            let now = Instant::now();
            delta = (now - self.old_time).as_secs_f64();
            self.old_time = now;
            self.elapsed_time += delta;
        }
        delta
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_start_returns_zero_first_delta() {
        let mut c = Clock::new(true);
        assert_eq!(c.get_delta(), 0.0);
        assert!(c.running);
    }

    #[test]
    fn delta_monotonic_after_start() {
        let mut c = Clock::new(true);
        let _ = c.get_delta();
        std::thread::sleep(std::time::Duration::from_millis(5));
        let d = c.get_delta();
        assert!(d > 0.0, "delta should be positive, got {}", d);
    }
}
