use super::BufferGeometry;
use crate::materials::Material;
use std::sync::Arc;

/// Point-list primitive. Each vertex is rendered as a single point.
/// Mirrors three.js's `Points`.
#[derive(Debug, Clone)]
pub struct Points {
    pub geometry: Arc<BufferGeometry>,
    pub material: Arc<Material>,
}

impl Points {
    pub fn new(geometry: BufferGeometry, material: Material) -> Self {
        Self {
            geometry: Arc::new(geometry),
            material: Arc::new(material),
        }
    }
}
