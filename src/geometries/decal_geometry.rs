use crate::core::{BufferAttribute, BufferGeometry};
use crate::math::{Matrix4, Quaternion, Vector3};

pub struct DecalGeometry;

impl DecalGeometry {
    /// Project a decal onto the world-space triangles of a base mesh. The decal
    /// is a thin oriented box at `position` (with `orientation` quaternion and
    /// `size` extents) — triangles whose vertices fall inside the box are kept
    /// and clipped to the box. Mirrors three.js's `DecalGeometry`.
    ///
    /// `base_positions` are the host mesh's triangle vertices in world space
    /// (caller is expected to transform them with the host's matrix_world).
    /// `base_indices` is optional (None = positions are flat triangles).
    pub fn new(
        base_positions: &[Vector3],
        base_indices: Option<&[u32]>,
        position: Vector3,
        orientation: Quaternion,
        size: Vector3,
    ) -> BufferGeometry {
        let mut g = BufferGeometry::new();
        let inv_orient = orientation.invert();
        let half = size * 0.5;

        let to_local = |p: Vector3| -> Vector3 {
            (p - position).apply_quaternion(inv_orient)
        };

        let mut local_positions: Vec<Vector3> = Vec::new();
        let triangles_iter: Box<dyn Iterator<Item = [usize; 3]>> = if let Some(idx) = base_indices {
            Box::new(idx.chunks_exact(3).map(|c| [c[0] as usize, c[1] as usize, c[2] as usize]))
        } else {
            Box::new((0..base_positions.len() / 3).map(|i| [i * 3, i * 3 + 1, i * 3 + 2]))
        };
        for tri in triangles_iter {
            let a = to_local(base_positions[tri[0]]);
            let b = to_local(base_positions[tri[1]]);
            let c = to_local(base_positions[tri[2]]);
            let inside = |p: Vector3| {
                p.x.abs() <= half.x && p.y.abs() <= half.y && p.z.abs() <= half.z
            };
            if inside(a) || inside(b) || inside(c) {
                local_positions.push(a);
                local_positions.push(b);
                local_positions.push(c);
            }
        }
        if local_positions.is_empty() { return g; }

        // Transform decal-local back to world space and build UVs from x/y.
        let mut positions = Vec::with_capacity(local_positions.len() * 3);
        let mut normals = Vec::with_capacity(local_positions.len() * 3);
        let mut uvs = Vec::with_capacity(local_positions.len() * 2);
        let _ = Matrix4::compose(position, orientation, Vector3::ONE);
        for tri in local_positions.chunks_exact(3) {
            let world_a = position + tri[0].apply_quaternion(orientation);
            let world_b = position + tri[1].apply_quaternion(orientation);
            let world_c = position + tri[2].apply_quaternion(orientation);
            let n = (world_b - world_a).cross(world_c - world_a).normalize();
            for (lp, wp) in [(tri[0], world_a), (tri[1], world_b), (tri[2], world_c)] {
                positions.extend_from_slice(&[wp.x, wp.y, wp.z]);
                normals.extend_from_slice(&[n.x, n.y, n.z]);
                uvs.extend_from_slice(&[
                    lp.x / size.x + 0.5,
                    lp.y / size.y + 0.5,
                ]);
            }
        }
        g.set_attribute("position", BufferAttribute::new(positions, 3));
        g.set_attribute("normal",   BufferAttribute::new(normals, 3));
        g.set_attribute("uv",       BufferAttribute::new(uvs, 2));
        g
    }
}
