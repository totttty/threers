use super::AnimationClip;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopMode {
    Once,
    Repeat,
    PingPong,
}

#[derive(Debug, Clone)]
pub struct AnimationAction {
    pub clip: AnimationClip,
    pub enabled: bool,
    pub weight: f32,
    pub time_scale: f32,
    pub loop_mode: LoopMode,
    time: f32,
    direction: f32,
}

impl AnimationAction {
    pub fn new(clip: AnimationClip) -> Self {
        Self {
            clip,
            enabled: true,
            weight: 1.0,
            time_scale: 1.0,
            loop_mode: LoopMode::Repeat,
            time: 0.0,
            direction: 1.0,
        }
    }

    pub fn play(&mut self) {
        self.enabled = true;
    }
    pub fn stop(&mut self) {
        self.enabled = false;
        self.time = 0.0;
    }
    pub fn pause(&mut self) {
        self.enabled = false;
    }

    pub fn current_time(&self) -> f32 {
        self.time
    }

    pub fn advance(&mut self, delta: f32) {
        if self.clip.duration <= 0.0 {
            return;
        }
        let scaled = delta * self.time_scale * self.direction;
        self.time += scaled;
        match self.loop_mode {
            LoopMode::Once => {
                if self.time > self.clip.duration {
                    self.time = self.clip.duration;
                    self.enabled = false;
                }
            }
            LoopMode::Repeat => {
                while self.time > self.clip.duration {
                    self.time -= self.clip.duration;
                }
                while self.time < 0.0 {
                    self.time += self.clip.duration;
                }
            }
            LoopMode::PingPong => {
                if self.time > self.clip.duration {
                    self.time = self.clip.duration - (self.time - self.clip.duration);
                    self.direction = -1.0;
                }
                if self.time < 0.0 {
                    self.time = -self.time;
                    self.direction = 1.0;
                }
            }
        }
    }
}
