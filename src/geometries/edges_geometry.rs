use std::collections::HashMap;
use crate::core::{BufferGeometry, BufferAttribute};
use crate::math::Vector3;

pub struct EdgesGeometry;

impl EdgesGeometry {
    /// Extract sharp edges from `source`: edges whose adjacent triangles' face
    /// normals differ by more than `threshold_angle` radians are emitted as a
    /// line-list. Border edges (only one adjacent triangle) are always emitted.
    /// Returns a non-indexed line geometry with `position` only.
    pub fn new(source: &BufferGeometry, threshold_angle: f32) -> BufferGeometry {
        let positions: Vec<Vector3> = match source.positions() {
            Some(it) => it.collect(),
            None => return empty_lines(),
        };

        let triangles: Vec<[u32; 3]> = match &source.index {
            Some(idx) => idx.chunks_exact(3).map(|c| [c[0], c[1], c[2]]).collect(),
            None => (0..positions.len() as u32 / 3)
                .map(|i| [i * 3, i * 3 + 1, i * 3 + 2])
                .collect(),
        };

        let mut keep: Vec<(u32, u32)> = Vec::new();
        let cos_thresh = threshold_angle.cos();

        let face_normal = |t: &[u32; 3]| -> Vector3 {
            let a = positions[t[0] as usize];
            let b = positions[t[1] as usize];
            let c = positions[t[2] as usize];
            (b - a).cross(c - a).normalize()
        };

        let mut normals: Vec<Vector3> = Vec::with_capacity(triangles.len());
        for t in &triangles { normals.push(face_normal(t)); }

        let edges_of = |t: &[u32; 3]| [
            ordered_pair(t[0], t[1]),
            ordered_pair(t[1], t[2]),
            ordered_pair(t[2], t[0]),
        ];

        // Track shared edges and their two triangles.
        let mut shared: HashMap<(u32, u32), (usize, Option<usize>)> = HashMap::new();
        for (ti, t) in triangles.iter().enumerate() {
            for e in edges_of(t) {
                let entry = shared.entry(e).or_insert((ti, None));
                if entry.0 != ti && entry.1.is_none() { entry.1 = Some(ti); }
            }
        }

        for (e, (t0, t1)) in shared.iter() {
            match t1 {
                None => keep.push(*e), // border edge
                Some(t1_idx) => {
                    let d = normals[*t0].dot(normals[*t1_idx]);
                    if d < cos_thresh { keep.push(*e); }
                }
            }
        }

        // Emit positions for each kept edge (two vertices per line segment).
        let mut out_positions = Vec::with_capacity(keep.len() * 6);
        for (a, b) in keep {
            let pa = positions[a as usize];
            let pb = positions[b as usize];
            out_positions.extend_from_slice(&[pa.x, pa.y, pa.z, pb.x, pb.y, pb.z]);
        }

        let mut geom = BufferGeometry::new();
        geom.set_attribute("position", BufferAttribute::new(out_positions, 3));
        geom
    }
}

pub struct WireframeGeometry;

impl WireframeGeometry {
    /// Every triangle edge as a line. Duplicates are removed.
    pub fn new(source: &BufferGeometry) -> BufferGeometry {
        let positions: Vec<Vector3> = match source.positions() {
            Some(it) => it.collect(),
            None => return empty_lines(),
        };
        let triangles: Vec<[u32; 3]> = match &source.index {
            Some(idx) => idx.chunks_exact(3).map(|c| [c[0], c[1], c[2]]).collect(),
            None => (0..positions.len() as u32 / 3)
                .map(|i| [i * 3, i * 3 + 1, i * 3 + 2])
                .collect(),
        };

        let mut seen: HashMap<(u32, u32), ()> = HashMap::new();
        for t in &triangles {
            for e in [ordered_pair(t[0], t[1]), ordered_pair(t[1], t[2]), ordered_pair(t[2], t[0])] {
                seen.entry(e).or_insert(());
            }
        }

        let mut out_positions = Vec::with_capacity(seen.len() * 6);
        for (a, b) in seen.keys() {
            let pa = positions[*a as usize];
            let pb = positions[*b as usize];
            out_positions.extend_from_slice(&[pa.x, pa.y, pa.z, pb.x, pb.y, pb.z]);
        }

        let mut geom = BufferGeometry::new();
        geom.set_attribute("position", BufferAttribute::new(out_positions, 3));
        geom
    }
}

fn ordered_pair(a: u32, b: u32) -> (u32, u32) {
    if a < b { (a, b) } else { (b, a) }
}

fn empty_lines() -> BufferGeometry {
    let mut g = BufferGeometry::new();
    g.set_attribute("position", BufferAttribute::new(Vec::new(), 3));
    g
}
