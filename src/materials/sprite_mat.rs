use std::sync::Arc;
use crate::math::Color;
use crate::textures::Texture;

/// Sprite billboard material. Mirrors three.js's `SpriteMaterial`.
#[derive(Debug, Clone)]
pub struct SpriteMaterial {
    pub color: Color,
    pub opacity: f32,
    pub rotation: f32,
    pub map: Option<Arc<Texture>>,
}

impl Default for SpriteMaterial {
    fn default() -> Self {
        Self { color: Color::WHITE, opacity: 1.0, rotation: 0.0, map: None }
    }
}

impl SpriteMaterial {
    pub fn new(color: Color) -> Self {
        Self { color, ..Default::default() }
    }

    pub fn with_map(mut self, map: Arc<Texture>) -> Self {
        self.map = Some(map);
        self
    }
}
