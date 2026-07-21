use crate::math::Color;
use crate::textures::Texture;
use std::sync::Arc;

/// MatCap (material capture) sampling: viewspace normal → UV into a matcap
/// texture. Matches three.js's `MeshMatcapMaterial`.
#[derive(Debug, Clone)]
pub struct MatcapMaterial {
    pub color: Color,
    pub matcap: Option<Arc<Texture>>,
    pub opacity: f32,
    pub wireframe: bool,
}

impl Default for MatcapMaterial {
    fn default() -> Self {
        Self {
            color: Color::WHITE,
            matcap: None,
            opacity: 1.0,
            wireframe: false,
        }
    }
}

impl MatcapMaterial {
    pub fn new(matcap: Arc<Texture>) -> Self {
        Self {
            matcap: Some(matcap),
            ..Default::default()
        }
    }
}
