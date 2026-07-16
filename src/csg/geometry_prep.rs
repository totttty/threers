//! Geometry helpers for CSG brushes (indexing, soup expansion, reference bins).

use crate::core::{BufferAttribute, BufferGeometry};

pub fn tri_count(geometry: &BufferGeometry) -> usize {
    if let Some(idx) = &geometry.index {
        idx.len() / 3
    } else {
        geometry
            .get_attribute("position")
            .map(|a| a.count() / 3)
            .unwrap_or(0)
    }
}

pub fn index_at(geometry: &BufferGeometry, tri: usize, corner: usize) -> usize {
    let i = tri * 3 + corner;
    if let Some(idx) = &geometry.index {
        idx[i] as usize
    } else {
        i
    }
}

/// Load triangle-soup positions from a `TCG1` export
/// (`tests/parity/export-*-geometry.mjs` / `export-csg-hierarchy-steps.mjs`).
pub fn load_positions_geometry_bin(raw: &[u8]) -> BufferGeometry {
    assert!(raw.len() >= 8 && &raw[0..4] == b"TCG1", "expected TCG1 geometry bin");
    let vert_count = u32::from_le_bytes(raw[4..8].try_into().expect("header")) as usize;
    let pos_bytes = &raw[8..8 + vert_count * 12];
    let mut positions = Vec::with_capacity(vert_count * 3);
    for chunk in pos_bytes.chunks_exact(4) {
        positions.push(f32::from_le_bytes(chunk.try_into().expect("f32")));
    }
    let mut geom = BufferGeometry::new();
    geom.set_attribute("position", BufferAttribute::new(positions, 3));
    geom
}

/// Expand indexed geometry to triangle soup (three.js `toNonIndexed`).
pub fn ensure_non_indexed(geometry: &mut BufferGeometry) {
    let Some(indices) = geometry.index.take() else {
        return;
    };
    let tri_len = indices.len();
    for attr in geometry.attributes.values_mut() {
        let item_size = attr.item_size;
        let mut data = vec![0.0f32; tri_len * item_size];
        for (dst_tri, &vi) in indices.iter().enumerate() {
            let src = vi as usize * item_size;
            let dst = dst_tri * item_size;
            data[dst..dst + item_size].copy_from_slice(&attr.array[src..src + item_size]);
        }
        attr.array = data;
    }
    geometry.bounding_box = None;
    geometry.bounding_sphere = None;
    geometry.geometry_version = geometry.geometry_version.wrapping_add(1);
    #[cfg(feature = "mesh-bvh")]
    {
        geometry.bounds_tree = None;
    }
}

pub fn ensure_index(geometry: &mut BufferGeometry) {
    if geometry.index.is_some() {
        return;
    }
    let count = geometry
        .get_attribute("position")
        .map(|a| a.count())
        .unwrap_or(0);
    let index: Vec<u32> = (0..count as u32).collect();
    geometry.set_index(index);
}

pub fn build_group_indices(geometry: &BufferGeometry) -> Vec<u16> {
    vec![0; tri_count(geometry)]
}

pub fn read_position(attr: &BufferAttribute, index: usize) -> crate::math::Vector3 {
    let i = index * attr.item_size;
    crate::math::Vector3::new(attr.array[i], attr.array[i + 1], attr.array[i + 2])
}
