use crate::core::{BufferAttribute, BufferGeometry, LineSegments, Object3D};
use crate::materials::LineBasicMaterial;
use crate::math::{Box3, Color, Vector3};

/// Wireframe box around an AABB. Mirrors three.js's `BoxHelper` (we accept a
/// Box3 directly instead of an `Object3D`'s bounding box).
pub struct BoxHelper;

impl BoxHelper {
    pub fn new(bb: &Box3, color: Color) -> Object3D {
        if bb.is_empty() {
            let mut g = BufferGeometry::new();
            g.set_attribute("position", BufferAttribute::new(Vec::new(), 3));
            let mat = LineBasicMaterial::new(color);
            return Object3D::line_segments(LineSegments::new(g, mat.into()));
        }
        let corners = [
            Vector3::new(bb.min.x, bb.min.y, bb.min.z),
            Vector3::new(bb.max.x, bb.min.y, bb.min.z),
            Vector3::new(bb.max.x, bb.max.y, bb.min.z),
            Vector3::new(bb.min.x, bb.max.y, bb.min.z),
            Vector3::new(bb.min.x, bb.min.y, bb.max.z),
            Vector3::new(bb.max.x, bb.min.y, bb.max.z),
            Vector3::new(bb.max.x, bb.max.y, bb.max.z),
            Vector3::new(bb.min.x, bb.max.y, bb.max.z),
        ];
        let edges = [
            (0,1),(1,2),(2,3),(3,0),
            (4,5),(5,6),(6,7),(7,4),
            (0,4),(1,5),(2,6),(3,7),
        ];
        let mut positions = Vec::with_capacity(edges.len() * 6);
        for (a, b) in edges {
            let pa = corners[a];
            let pb = corners[b];
            positions.extend_from_slice(&[pa.x, pa.y, pa.z, pb.x, pb.y, pb.z]);
        }
        let mut g = BufferGeometry::new();
        g.set_attribute("position", BufferAttribute::new(positions, 3));
        let mat = LineBasicMaterial::new(color);
        Object3D::line_segments(LineSegments::new(g, mat.into()))
    }
}
