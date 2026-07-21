use super::BufferGeometry;
use crate::materials::Material;
use std::sync::Arc;

/// A drawable: geometry + material. The renderer caches per-mesh GPU buffers
/// using identity of these Arcs.
#[derive(Debug, Clone)]
pub struct Mesh {
    pub geometry: Arc<BufferGeometry>,
    pub material: Arc<Material>,
}

impl Mesh {
    pub fn new(geometry: BufferGeometry, material: Material) -> Self {
        Self {
            geometry: Arc::new(geometry),
            material: Arc::new(material),
        }
    }

    pub fn from_arc(geometry: Arc<BufferGeometry>, material: Arc<Material>) -> Self {
        Self { geometry, material }
    }
}
