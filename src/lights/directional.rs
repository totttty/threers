use super::ShadowSettings;
use crate::math::{Color, Vector3};

/// Light shining from `direction` toward the origin (in three.js, the
/// `DirectionalLight.position` is the source, looking at `target`; we use
/// a single `direction` for simplicity, transformed by `matrix_world`).
#[derive(Debug, Clone, Copy)]
pub struct DirectionalLight {
    pub color: Color,
    pub intensity: f32,
    /// Direction from the light *to* the target, in object-local space.
    /// Defaults to `(0, -1, 0)` — straight down.
    pub direction: Vector3,
    pub cast_shadow: bool,
    pub shadow: ShadowSettings,
}

impl Default for DirectionalLight {
    fn default() -> Self {
        Self::new(Color::WHITE, 1.0)
    }
}

impl DirectionalLight {
    pub fn new(color: Color, intensity: f32) -> Self {
        Self {
            color,
            intensity,
            direction: Vector3::new(0.0, -1.0, 0.0),
            cast_shadow: false,
            shadow: ShadowSettings::default(),
        }
    }

    pub fn with_direction(mut self, direction: Vector3) -> Self {
        self.direction = direction;
        self
    }
}
