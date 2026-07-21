use crate::materials::Material;
use std::sync::Arc;

/// Camera-facing billboard. Mirrors three.js's `Sprite`. The renderer builds
/// the geometry on the fly — Sprite carries no `BufferGeometry`, only a material.
#[derive(Debug, Clone)]
pub struct Sprite {
    pub material: Arc<Material>,
    pub center: crate::math::Vector2,
}

impl Sprite {
    pub fn new(material: Material) -> Self {
        Self {
            material: Arc::new(material),
            center: crate::math::Vector2::new(0.5, 0.5),
        }
    }
}
