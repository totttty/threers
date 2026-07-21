use crate::core::{BufferAttribute, BufferGeometry, LineSegments, Object3D};
use crate::materials::LineBasicMaterial;
use crate::math::{Color, Vector3};

/// Line from origin along a direction, with two short fletches at the tip.
/// Approximates three.js's `ArrowHelper`.
pub struct ArrowHelper;

impl ArrowHelper {
    pub fn new(direction: Vector3, origin: Vector3, length: f32, color: Color) -> Object3D {
        let dir = direction.normalize();
        let tip = origin + dir * length;
        let head_len = length * 0.2;

        // Build a fletch in the plane perpendicular to `dir`.
        let up = if dir.y.abs() < 0.99 {
            Vector3::new(0.0, 1.0, 0.0)
        } else {
            Vector3::new(1.0, 0.0, 0.0)
        };
        let side = dir.cross(up).normalize();
        let back = -dir;
        let f0 = tip + (back + side * 0.5).normalize() * head_len;
        let f1 = tip + (back - side * 0.5).normalize() * head_len;

        let positions = vec![
            origin.x, origin.y, origin.z, tip.x, tip.y, tip.z, tip.x, tip.y, tip.z, f0.x, f0.y,
            f0.z, tip.x, tip.y, tip.z, f1.x, f1.y, f1.z,
        ];
        let mut g = BufferGeometry::new();
        g.set_attribute("position", BufferAttribute::new(positions, 3));
        let mat = LineBasicMaterial::new(color);
        Object3D::line_segments(LineSegments::new(g, mat.into()))
    }
}
