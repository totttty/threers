use crate::math::Color;

/// Stepped/cel-shaded NPR material. Matches three.js's `MeshToonMaterial`.
/// The toon "gradient map" is approximated by `steps` (number of bands).
#[derive(Debug, Clone, Copy)]
pub struct ToonMaterial {
    pub color: Color,
    pub emissive: Color,
    pub opacity: f32,
    pub steps: u32,
    pub wireframe: bool,
    pub side: u32,
}

impl Default for ToonMaterial {
    fn default() -> Self {
        Self { color: Color::WHITE, emissive: Color::BLACK, opacity: 1.0, steps: 3, wireframe: false, side: 0 }
    }
}

impl ToonMaterial {
    pub fn new(color: Color) -> Self { Self { color, ..Default::default() } }
}
