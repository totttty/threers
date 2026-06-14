use crate::math::Color;
use super::ShadowSettings;

/// Omnidirectional light radiating from `position` (taken from the parent
/// `Object3D`'s world matrix). Falloff matches three.js:
/// `1 / max(d^decay, 0.01)` with a smooth cutoff at `distance` when `distance > 0`.
#[derive(Debug, Clone, Copy)]
pub struct PointLight {
    pub color: Color,
    pub intensity: f32,
    /// Cutoff distance. 0 means infinite range.
    pub distance: f32,
    pub decay: f32,
    pub cast_shadow: bool,
    pub shadow: ShadowSettings,
}

impl Default for PointLight {
    fn default() -> Self { Self::new(Color::WHITE, 1.0) }
}

impl PointLight {
    pub fn new(color: Color, intensity: f32) -> Self {
        Self {
            color, intensity,
            distance: 0.0,
            decay: 2.0,
            cast_shadow: false,
            shadow: ShadowSettings::default(),
        }
    }

    pub fn with_distance(mut self, distance: f32) -> Self {
        self.distance = distance;
        self
    }

    pub fn with_decay(mut self, decay: f32) -> Self {
        self.decay = decay;
        self
    }
}
