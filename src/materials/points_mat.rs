use crate::math::Color;

/// Point material. Mirrors three.js's `PointsMaterial`. `size_attenuation`
/// scales size by 1/depth in perspective; ignored under orthographic.
#[derive(Debug, Clone, Copy)]
pub struct PointsMaterial {
    pub color: Color,
    pub opacity: f32,
    pub size: f32,
    pub size_attenuation: bool,
}

impl Default for PointsMaterial {
    fn default() -> Self {
        Self {
            color: Color::WHITE,
            opacity: 1.0,
            size: 1.0,
            size_attenuation: true,
        }
    }
}

impl PointsMaterial {
    pub fn new(color: Color, size: f32) -> Self {
        Self {
            color,
            size,
            ..Default::default()
        }
    }
}
