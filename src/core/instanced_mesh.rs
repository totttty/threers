use std::sync::Arc;
use crate::core::BufferGeometry;
use crate::materials::Material;
use crate::math::Matrix4;

/// Mesh drawn many times with per-instance transforms. Mirrors three.js's
/// `InstancedMesh`. The renderer uploads `transforms` into a per-instance
/// vertex buffer (mat4 = 4 vec4 attrs).
#[derive(Debug, Clone)]
pub struct InstancedMesh {
    pub geometry: Arc<BufferGeometry>,
    pub material: Arc<Material>,
    pub transforms: Vec<Matrix4>,
}

impl InstancedMesh {
    pub fn new(geometry: BufferGeometry, material: Material, count: usize) -> Self {
        Self {
            geometry: Arc::new(geometry),
            material: Arc::new(material),
            transforms: vec![Matrix4::identity(); count],
        }
    }

    pub fn set_matrix_at(&mut self, i: usize, m: Matrix4) {
        if i < self.transforms.len() {
            self.transforms[i] = m;
        }
    }

    pub fn count(&self) -> usize { self.transforms.len() }
}
