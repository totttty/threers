use crate::math::Vector3;

/// A ray-hit result from BVH traversal in local geometry space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BvhHit {
    pub distance: f32,
    pub point: Vector3,
    pub face_index: usize,
    pub uv: Vector3,
}

impl BvhHit {
    pub fn new(distance: f32, point: Vector3, face_index: usize, uv: Vector3) -> Self {
        Self {
            distance,
            point,
            face_index,
            uv,
        }
    }
}
