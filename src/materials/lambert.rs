use crate::math::Color;

/// Diffuse-only material. Matches three.js's `MeshLambertMaterial`.
#[derive(Debug, Clone, Copy)]
pub struct LambertMaterial {
    pub color: Color,
    pub emissive: Color,
    pub opacity: f32,
    pub wireframe: bool,
    pub side: u32,
}

impl Default for LambertMaterial {
    fn default() -> Self {
        Self {
            color: Color::WHITE,
            emissive: Color::BLACK,
            opacity: 1.0,
            wireframe: false,
            side: 0,
        }
    }
}

impl LambertMaterial {
    pub fn new(color: Color) -> Self {
        Self {
            color,
            ..Default::default()
        }
    }

    pub fn with_emissive(mut self, c: Color) -> Self {
        self.emissive = c;
        self
    }

    pub fn with_opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity;
        self
    }
}
