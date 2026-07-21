use crate::core::{BufferAttribute, BufferGeometry};
use std::f32::consts::PI;

pub struct CapsuleGeometry;

impl CapsuleGeometry {
    /// Cylinder along Y with hemispherical end caps. Matches three.js's
    /// `CapsuleGeometry(radius, length, capSegments, radialSegments)`. `length`
    /// is the cylinder length between the cap centers.
    pub fn new(
        radius: f32,
        length: f32,
        cap_segments: usize,
        radial_segments: usize,
    ) -> BufferGeometry {
        let cs = cap_segments.max(1);
        let rs = radial_segments.max(3);
        let half_l = length * 0.5;

        let mut positions = Vec::new();
        let mut normals = Vec::new();
        let mut uvs = Vec::new();
        let mut indices = Vec::new();

        // Build rings: top hemisphere (cs rings), cylinder (2 rings shared with caps),
        // bottom hemisphere (cs rings). Total rings = 2*cs + 2.
        // We'll emit (rings_total + 1) rings of (rs + 1) vertices.
        let total_rings = 2 * cs + 1;
        for ring in 0..=total_rings {
            let (y, ring_radius) = if ring <= cs {
                // top hemisphere: phi from 0 (pole) at ring=0 to PI/2 (equator) at ring=cs
                let phi = (ring as f32 / cs as f32) * (PI * 0.5);
                let r = radius * phi.sin();
                let y = half_l + radius * phi.cos();
                (y, r)
            } else {
                // bottom hemisphere mirror; ring index 'k' from cs+1..=total_rings
                let k = ring - cs;
                let phi = (k as f32 / cs as f32) * (PI * 0.5);
                let r = radius * phi.cos();
                let y = -half_l - radius * phi.sin();
                (y, r)
            };

            for i in 0..=rs {
                let u = i as f32 / rs as f32;
                let theta = u * PI * 2.0;
                let (s, c) = theta.sin_cos();
                let x = ring_radius * c;
                let z = ring_radius * s;
                positions.extend_from_slice(&[x, y, z]);
                let (nx, ny, nz) = if ring < cs {
                    let cy = half_l;
                    let dx = x;
                    let dy = y - cy;
                    let dz = z;
                    let l = (dx * dx + dy * dy + dz * dz).sqrt().max(1e-8);
                    (dx / l, dy / l, dz / l)
                } else if ring > cs {
                    let cy = -half_l;
                    let dx = x;
                    let dy = y - cy;
                    let dz = z;
                    let l = (dx * dx + dy * dy + dz * dz).sqrt().max(1e-8);
                    (dx / l, dy / l, dz / l)
                } else {
                    (c, 0.0, s)
                };
                normals.extend_from_slice(&[nx, ny, nz]);
                uvs.extend_from_slice(&[u, 1.0 - ring as f32 / total_rings as f32]);
            }
        }

        let row = (rs + 1) as u32;
        for r in 0..total_rings as u32 {
            for i in 0..rs as u32 {
                let a = r * row + i;
                let b = (r + 1) * row + i;
                let c = (r + 1) * row + i + 1;
                let d = r * row + i + 1;
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

    pub fn default_(radius: f32, length: f32) -> BufferGeometry {
        Self::new(radius, length, 4, 16)
    }
}
