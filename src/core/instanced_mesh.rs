use crate::core::BufferGeometry;
use crate::materials::Material;
use crate::math::Color;
use crate::math::Matrix4;
use std::sync::Arc;

/// Mesh drawn many times with per-instance transforms. Mirrors three.js's
/// `InstancedMesh`. The renderer uploads `transforms` into a per-instance
/// vertex buffer (mat4 = 4 vec4 attrs).
#[derive(Debug, Clone)]
pub struct InstancedMesh {
    pub geometry: Arc<BufferGeometry>,
    pub material: Arc<Material>,
    pub transforms: Vec<Matrix4>,
    pub colors: Vec<Color>,
}

impl InstancedMesh {
    pub fn new(geometry: BufferGeometry, material: Material, count: usize) -> Self {
        Self {
            geometry: Arc::new(geometry),
            material: Arc::new(material),
            transforms: vec![Matrix4::identity(); count],
            colors: vec![Color::new(1.0, 1.0, 1.0); count],
        }
    }

    pub fn set_matrix_at(&mut self, i: usize, m: Matrix4) {
        if i < self.transforms.len() {
            self.transforms[i] = m;
        }
    }

    pub fn set_color_at(&mut self, i: usize, color: Color) {
        if i < self.colors.len() {
            self.colors[i] = color;
        }
    }

    pub fn count(&self) -> usize {
        self.transforms.len()
    }
}
