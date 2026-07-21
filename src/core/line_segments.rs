use super::BufferGeometry;
use crate::materials::Material;
use std::sync::Arc;

/// Line-list primitive. Each two consecutive vertices form a segment.
/// Mirrors three.js's `LineSegments`.
#[derive(Debug, Clone)]
pub struct LineSegments {
    pub geometry: Arc<BufferGeometry>,
    pub material: Arc<Material>,
}

impl LineSegments {
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
