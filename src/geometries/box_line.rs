use crate::core::{BufferAttribute, BufferGeometry};
use crate::math::Vector3;

pub struct BoxLineGeometry;

impl BoxLineGeometry {
    /// 12-edge wireframe box. Matches three.js's `BoxLineGeometry`.
    pub fn new(width: f32, height: f32, depth: f32) -> BufferGeometry {
        let hx = width * 0.5;
        let hy = height * 0.5;
        let hz = depth * 0.5;
        let v = [
            Vector3::new(-hx, -hy, -hz),
            Vector3::new(hx, -hy, -hz),
            Vector3::new(hx, hy, -hz),
            Vector3::new(-hx, hy, -hz),
            Vector3::new(-hx, -hy, hz),
            Vector3::new(hx, -hy, hz),
            Vector3::new(hx, hy, hz),
            Vector3::new(-hx, hy, hz),
        ];
        let edges = [
            (0, 1),
            (1, 2),
            (2, 3),
            (3, 0),
            (4, 5),
            (5, 6),
            (6, 7),
            (7, 4),
            (0, 4),
            (1, 5),
            (2, 6),
            (3, 7),
        ];
        let mut positions = Vec::with_capacity(edges.len() * 6);
        for (a, b) in edges {
            let pa = v[a];
            let pb = v[b];
            positions.extend_from_slice(&[pa.x, pa.y, pa.z, pb.x, pb.y, pb.z]);
        }
        let mut g = BufferGeometry::new();
        g.set_attribute("position", BufferAttribute::new(positions, 3));
        g
    }
}
