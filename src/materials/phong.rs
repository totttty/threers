use crate::math::Color;

/// Diffuse + Blinn-Phong specular + emissive. Matches three.js's `MeshPhongMaterial`.
#[derive(Debug, Clone, Copy)]
pub struct PhongMaterial {
    pub color: Color,
    pub emissive: Color,
    pub specular: Color,
    pub shininess: f32,
    pub opacity: f32,
    pub wireframe: bool,
    pub side: u32,
}

impl Default for PhongMaterial {
    fn default() -> Self {
        Self {
            color: Color::WHITE,
            emissive: Color::BLACK,
            specular: Color::new(0.07, 0.07, 0.07),
            shininess: 30.0,
            opacity: 1.0,
            wireframe: false,
            side: 0,
        }
    }
}

impl PhongMaterial {
    pub fn new(color: Color) -> Self {
        Self {
            color,
            ..Default::default()
        }
    }

    pub fn with_specular(mut self, specular: Color, shininess: f32) -> Self {
        self.specular = specular;
        self.shininess = shininess;
        self
    }

    pub fn with_emissive(mut self, c: Color) -> Self {
        self.emissive = c;
        self
    }
}
