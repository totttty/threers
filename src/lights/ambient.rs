use crate::math::Color;

#[derive(Debug, Clone, Copy)]
pub struct AmbientLight {
    pub color: Color,
    pub intensity: f32,
}

impl Default for AmbientLight {
    fn default() -> Self { Self::new(Color::WHITE, 1.0) }
}

impl AmbientLight {
    pub const fn new(color: Color, intensity: f32) -> Self {
        Self { color, intensity }
    }
}
