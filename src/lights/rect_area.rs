use crate::math::Color;

/// Area light shaped like a rectangle. Matches three.js's API surface.
/// The renderer does not implement the full LTC-based rect-area shading yet;
/// it's treated as a soft directional approximation until the BRDF lands.
#[derive(Debug, Clone, Copy)]
pub struct RectAreaLight {
    pub color: Color,
    pub intensity: f32,
    pub width: f32,
    pub height: f32,
}

impl Default for RectAreaLight {
    fn default() -> Self { Self::new(Color::WHITE, 1.0, 10.0, 10.0) }
}

impl RectAreaLight {
    pub const fn new(color: Color, intensity: f32, width: f32, height: f32) -> Self {
        Self { color, intensity, width, height }
    }
}
