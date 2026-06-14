use crate::cameras::Camera;
use crate::core::{BufferAttribute, BufferGeometry, LineSegments, Object3D};
use crate::materials::LineBasicMaterial;
use crate::math::{Color, Matrix4, Vector3};

/// Wireframe frustum for a camera. Mirrors three.js's `CameraHelper`.
pub struct CameraHelper;

impl CameraHelper {
    pub fn new(camera: &dyn Camera, color: Color) -> Object3D {
        // Unproject the eight NDC corners back into world space.
        let proj = camera.projection_matrix();
        let view = camera.view_matrix();
        let inv = proj.multiply(&view).invert();
        let ndc = [
            Vector3::new(-1.0, -1.0, -1.0),
            Vector3::new( 1.0, -1.0, -1.0),
            Vector3::new( 1.0,  1.0, -1.0),
            Vector3::new(-1.0,  1.0, -1.0),
            Vector3::new(-1.0, -1.0,  1.0),
            Vector3::new( 1.0, -1.0,  1.0),
            Vector3::new( 1.0,  1.0,  1.0),
            Vector3::new(-1.0,  1.0,  1.0),
        ];
        let corners: Vec<Vector3> = ndc.iter().map(|p| unproject(*p, &inv)).collect();
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

fn unproject(p: Vector3, inv: &Matrix4) -> Vector3 {
    p.apply_matrix4(inv)
}
