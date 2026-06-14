use crate::math::{Color, Vector3};
use super::ShadowSettings;

/// Conical light from `position` along `direction`, with an outer half-angle
/// and a `penumbra` (0..=1) fading region at the cone's edge.
#[derive(Debug, Clone, Copy)]
pub struct SpotLight {
    pub color: Color,
    pub intensity: f32,
    pub direction: Vector3,
    pub distance: f32,
    pub decay: f32,
    /// Half-angle of the cone in radians.
    pub angle: f32,
    /// Soft edge as a fraction of the angle. 0 = hard, 1 = entire cone.
    pub penumbra: f32,
    pub cast_shadow: bool,
    pub shadow: ShadowSettings,
}

impl Default for SpotLight {
    fn default() -> Self { Self::new(Color::WHITE, 1.0) }
}

impl SpotLight {
    pub fn new(color: Color, intensity: f32) -> Self {
        Self {
            color,
            intensity,
            direction: Vector3::new(0.0, -1.0, 0.0),
            distance: 0.0,
            decay: 2.0,
            angle: std::f32::consts::FRAC_PI_4,
            penumbra: 0.0,
            cast_shadow: false,
            shadow: ShadowSettings::default(),
        }
    }
}
