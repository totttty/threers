use std::f32::consts::PI;
use crate::core::{BufferGeometry, BufferAttribute};
use crate::math::Vector3;

pub struct PolyhedronGeometry;

impl PolyhedronGeometry {
    /// Triangulated polyhedron projected onto a sphere of `radius`. `vertices`
    /// is a flat array (x, y, z, ...). `indices` references triangles into that array.
    /// `detail` ≥ 1 recursively subdivides each face.
    pub fn new(vertices: &[f32], indices: &[u32], radius: f32, detail: usize) -> BufferGeometry {
        // Inflate vertices into Vec<Vector3>.
        let src_verts: Vec<Vector3> = vertices
            .chunks_exact(3)
            .map(|c| Vector3::new(c[0], c[1], c[2]))
            .collect();

        // Subdivided vertex list and triangle indices.
        let mut out_verts: Vec<Vector3> = Vec::new();
        let mut out_tris: Vec<[u32; 3]> = Vec::new();

        // Subdivide each source triangle. three.js uses `cols = detail + 1`.
        let cols = detail + 1;
        for tri in indices.chunks_exact(3) {
            let a = src_verts[tri[0] as usize];
            let b = src_verts[tri[1] as usize];
            let c = src_verts[tri[2] as usize];

            // Build a row-major grid of (cols+1) rows. Row i has (cols - i + 1) vertices.
            // Row 0 is along edge AB (i=0), row `cols` is the single vertex C.
            let mut row_indices: Vec<Vec<u32>> = Vec::with_capacity(cols + 1);
            for i in 0..=cols {
                let mut row = Vec::with_capacity(cols - i + 1);
                let aj = lerp_v3(a, c, i as f32 / cols as f32);
                let bj = lerp_v3(b, c, i as f32 / cols as f32);
                let rows_in_this = cols - i;
                for j in 0..=rows_in_this {
                    let v = if rows_in_this == 0 { aj } else { lerp_v3(aj, bj, j as f32 / rows_in_this as f32) };
                    out_verts.push(v);
                    row.push((out_verts.len() - 1) as u32);
                }
                row_indices.push(row);
            }

            // Emit triangles between consecutive rows.
            for i in 0..cols {
                let upper = &row_indices[i];
                let lower = &row_indices[i + 1];
                for j in 0..(2 * (cols - i) - 1) {
                    let k = j / 2;
                    if j % 2 == 0 {
                        out_tris.push([upper[k + 1], lower[k], upper[k]]);
                    } else {
                        out_tris.push([upper[k + 1], lower[k + 1], lower[k]]);
                    }
                }
            }
        }

        // three.js PolyhedronGeometry always emits non-indexed triangle soup.
        let mut positions = Vec::with_capacity(out_tris.len() * 9);
        let mut normals = Vec::with_capacity(out_tris.len() * 9);
        let mut uvs = Vec::with_capacity(out_tris.len() * 6);
        for tri in &out_tris {
            let mut pts = [Vector3::ZERO; 3];
            for (j, &idx) in tri.iter().enumerate() {
                let n = out_verts[idx as usize].normalize();
                pts[j] = n * radius;
            }
            let face_n = if detail == 0 {
                let e1 = pts[1] - pts[0];
                let e2 = pts[2] - pts[0];
                let mut n = e1.cross(e2);
                if n.length_sq() > 1e-12 {
                    n = n.normalize();
                }
                n
            } else {
                Vector3::ZERO
            };
            for (j, p) in pts.iter().enumerate() {
                positions.extend_from_slice(&[p.x, p.y, p.z]);
                let n = if detail == 0 {
                    face_n
                } else {
                    out_verts[tri[j] as usize].normalize()
                };
                normals.extend_from_slice(&[n.x, n.y, n.z]);
                let u = p.z.atan2(-p.x) / (PI * 2.0) + 0.5;
                let incl = (-p.y).atan2((p.x * p.x + p.z * p.z).sqrt());
                let v = 1.0 - (incl / PI + 0.5);
                uvs.extend_from_slice(&[u, v]);
            }
        }
        let mut geom = BufferGeometry::new();
        geom.set_attribute("position", BufferAttribute::new(positions, 3));
        geom.set_attribute("normal", BufferAttribute::new(normals, 3));
        geom.set_attribute("uv", BufferAttribute::new(uvs, 2));
        geom
    }
}

fn lerp_v3(a: Vector3, b: Vector3, t: f32) -> Vector3 { a + (b - a) * t }

pub struct TetrahedronGeometry;
impl TetrahedronGeometry {
    pub fn new(radius: f32, detail: usize) -> BufferGeometry {
        let v = [
            1.0,  1.0,  1.0,
           -1.0, -1.0,  1.0,
           -1.0,  1.0, -1.0,
            1.0, -1.0, -1.0,
        ];
        let i = [2u32, 1, 0,  0, 3, 2,  1, 3, 0,  2, 3, 1];
        PolyhedronGeometry::new(&v, &i, radius, detail)
    }
}

pub struct OctahedronGeometry;
impl OctahedronGeometry {
    pub fn new(radius: f32, detail: usize) -> BufferGeometry {
        let v = [
            1.0, 0.0, 0.0, -1.0, 0.0, 0.0,
            0.0, 1.0, 0.0,  0.0,-1.0, 0.0,
            0.0, 0.0, 1.0,  0.0, 0.0,-1.0,
        ];
        let i = [
            0u32, 2, 4,  0, 4, 3,  0, 3, 5,  0, 5, 2,
            1, 2, 5,  1, 5, 3,  1, 3, 4,  1, 4, 2,
        ];
        PolyhedronGeometry::new(&v, &i, radius, detail)
    }
}

pub struct IcosahedronGeometry;
impl IcosahedronGeometry {
    pub fn new(radius: f32, detail: usize) -> BufferGeometry {
        let t = (1.0 + 5f32.sqrt()) / 2.0;
        let v = [
            -1.0,  t, 0.0,   1.0,  t, 0.0,  -1.0, -t, 0.0,   1.0, -t, 0.0,
             0.0, -1.0,  t,  0.0,  1.0,  t,  0.0, -1.0, -t,  0.0,  1.0, -t,
             t,  0.0, -1.0,  t,  0.0,  1.0, -t,  0.0, -1.0, -t,  0.0,  1.0,
        ];
        let i = [
            0u32, 11, 5,  0, 5, 1,  0, 1, 7,  0, 7, 10,  0, 10, 11,
            1, 5, 9,  5, 11, 4, 11, 10, 2, 10, 7, 6,  7, 1, 8,
            3, 9, 4,  3, 4, 2,  3, 2, 6,  3, 6, 8,  3, 8, 9,
            4, 9, 5,  2, 4, 11, 6, 2, 10, 8, 6, 7,  9, 8, 1,
        ];
        PolyhedronGeometry::new(&v, &i, radius, detail)
    }
}

pub struct DodecahedronGeometry;
impl DodecahedronGeometry {
    pub fn new(radius: f32, detail: usize) -> BufferGeometry {
        let t = (1.0 + 5f32.sqrt()) / 2.0;
        let r = 1.0 / t;
        let v = [
            // (+-1, +-1, +-1)
            -1.0, -1.0, -1.0,  -1.0, -1.0,  1.0,
            -1.0,  1.0, -1.0,  -1.0,  1.0,  1.0,
             1.0, -1.0, -1.0,   1.0, -1.0,  1.0,
             1.0,  1.0, -1.0,   1.0,  1.0,  1.0,
            // (0, +-1/phi, +-phi)
             0.0, -r, -t,  0.0, -r,  t,
             0.0,  r, -t,  0.0,  r,  t,
            // (+-1/phi, +-phi, 0)
            -r, -t,  0.0,  -r,  t,  0.0,
             r, -t,  0.0,   r,  t,  0.0,
            // (+-phi, 0, +-1/phi)
            -t,  0.0, -r,   t,  0.0, -r,
            -t,  0.0,  r,   t,  0.0,  r,
        ];
        let i = [
            3u32, 11, 7,  3, 7, 15,  3, 15, 13,
            7, 19, 17, 7, 17, 6,  7, 6, 15,
            17, 4, 8,  17, 8, 10, 17, 10, 6,
            8, 0, 16,  8, 16, 2, 8, 2, 10,
            0, 12, 1,  0, 1, 18, 0, 18, 16,
            6, 10, 2,  6, 2, 13, 6, 13, 15,
            2, 16, 18, 2, 18, 3, 2, 3, 13,
            18, 1, 9,  18, 9, 11, 18, 11, 3,
            4, 14, 12, 4, 12, 0, 4, 0, 8,
            11, 9, 5,  11, 5, 19, 11, 19, 7,
            19, 5, 14, 19, 14, 4, 19, 4, 17,
            1, 12, 14, 1, 14, 5,  1, 5, 9,
        ];
        PolyhedronGeometry::new(&v, &i, radius, detail)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icosahedron_detail0_matches_threejs() {
        let g = IcosahedronGeometry::new(1.1, 0);
        let pos = g.get_attribute("position").expect("position");
        assert_eq!(pos.count(), 60, "20 triangles × 3 vertices");
        let data = &pos.array;
        let expected: [f32; 9] = [
            -0.9357, 0.0, 0.5783, 0.0, 0.5783, 0.9357, -0.5783, 0.9357, 0.0,
        ];
        for (i, (&a, &b)) in data[..9].iter().zip(expected.iter()).enumerate() {
            assert!((a - b).abs() < 0.001, "pos[{i}] = {a}, expected {b}");
        }
    }
}
