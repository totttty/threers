use crate::core::{BufferAttribute, BufferGeometry};
use crate::math::Vector3;
use std::f32::consts::PI;

pub struct TorusKnotGeometry;

impl TorusKnotGeometry {
    /// (p, q)-torus knot. p windings around the symmetry axis; q around the tube.
    pub fn new(
        radius: f32,
        tube: f32,
        tubular_segments: usize,
        radial_segments: usize,
        p: u32,
        q: u32,
    ) -> BufferGeometry {
        let ts = tubular_segments.max(3);
        let rs = radial_segments.max(3);

        let mut positions = Vec::new();
        let mut normals = Vec::new();
        let mut uvs = Vec::new();
        let mut indices = Vec::new();

        let pos_on_curve = |u: f32| -> Vector3 {
            let cu = u.cos();
            let su = u.sin();
            let qou = q as f32 / p as f32 * u;
            let cs = qou.cos();
            Vector3::new(
                radius * (2.0 + cs) * 0.5 * cu,
                radius * (2.0 + cs) * 0.5 * su,
                radius * qou.sin() * 0.5,
            )
        };

        for i in 0..=ts {
            let u = i as f32 / ts as f32 * p as f32 * PI * 2.0;
            let p1 = pos_on_curve(u);
            let p2 = pos_on_curve(u + 0.01);
            let t = (p2 - p1).normalize();
            let n = (p2 + p1).normalize();
            let b = t.cross(n).normalize();
            let n = b.cross(t).normalize();

            for j in 0..=rs {
                let v = j as f32 / rs as f32 * PI * 2.0;
                let cx = -tube * v.cos();
                let cy = tube * v.sin();
                let pos = Vector3::new(
                    p1.x + cx * n.x + cy * b.x,
                    p1.y + cx * n.y + cy * b.y,
                    p1.z + cx * n.z + cy * b.z,
                );
                positions.extend_from_slice(&[pos.x, pos.y, pos.z]);
                let nrm = (pos - p1).normalize();
                normals.extend_from_slice(&[nrm.x, nrm.y, nrm.z]);
                uvs.extend_from_slice(&[i as f32 / ts as f32, j as f32 / rs as f32]);
            }
        }

        let row = (rs + 1) as u32;
        for i in 1..=ts as u32 {
            for j in 1..=rs as u32 {
                let a = (row * (i - 1)) + j - 1;
                let b = (row * i) + j - 1;
                let c = (row * i) + j;
                let d = (row * (i - 1)) + j;
                indices.extend_from_slice(&[a, b, d]);
                indices.extend_from_slice(&[b, c, d]);
            }
        }

        let mut geom = BufferGeometry::new();
        geom.set_attribute("position", BufferAttribute::new(positions, 3));
        geom.set_attribute("normal", BufferAttribute::new(normals, 3));
        geom.set_attribute("uv", BufferAttribute::new(uvs, 2));
        geom.set_index(indices);
        geom
    }

    pub fn default_(radius: f32, tube: f32) -> BufferGeometry {
        Self::new(radius, tube, 64, 8, 2, 3)
    }
}
