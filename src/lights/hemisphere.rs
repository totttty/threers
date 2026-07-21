use crate::math::Color;

/// Sky/ground gradient light. Sky color blends with ground color along the
/// surface normal: `n.y > 0` picks up sky; `n.y < 0` picks up ground.
#[derive(Debug, Clone, Copy)]
pub struct HemisphereLight {
    pub sky_color: Color,
    pub ground_color: Color,
    pub intensity: f32,
}

impl Default for HemisphereLight {
    fn default() -> Self {
        Self::new(Color::WHITE, Color::BLACK, 1.0)
    }
}

impl HemisphereLight {
    pub const fn new(sky_color: Color, ground_color: Color, intensity: f32) -> Self {
        Self {
            sky_color,
            ground_color,
            intensity,
        }
    }
}
