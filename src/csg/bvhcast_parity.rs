//! BVH / bvhcast parity helpers for CSG split-order debugging.

use crate::core::BufferGeometry;
use crate::mesh_bvh::{BuildOptions, SerializedMeshBvh};

use super::brush::CsgBrush;
use super::geometry_prep::{ensure_index, ensure_non_indexed};
use super::hit_side::matrix_b_into_a;

pub fn prepare_brush_geometry(geometry: &mut BufferGeometry) {
    ensure_non_indexed(geometry);
    ensure_index(geometry);
    if geometry.bounds_tree.is_none() {
        geometry.compute_bounds_tree(BuildOptions {
            max_leaf_tris: 3,
            indirect: true,
            ..Default::default()
        });
    }
}

pub fn serialized_bvh(brush: &CsgBrush) -> SerializedMeshBvh {
    SerializedMeshBvh::from_bvh(&brush.bounds_tree())
}

pub fn bvhcast_pairs(a: &CsgBrush, b: &CsgBrush) -> Vec<(usize, usize)> {
    let matrix = matrix_b_into_a(&a.matrix_world, &b.matrix_world);
    a.bounds_tree().bvhcast(&b.bounds_tree(), &matrix)
}

pub fn pair_overlap(
    native: &[(usize, usize)],
    reference: &[(usize, usize)],
) -> (usize, usize, usize) {
    use std::collections::HashSet;
    let ref_set: HashSet<_> = reference.iter().copied().collect();
    let native_set: HashSet<_> = native.iter().copied().collect();
    let shared = native_set.intersection(&ref_set).count();
    (native_set.len(), ref_set.len(), shared)
}

pub fn bvh_buffers_match(a: &SerializedMeshBvh, b: &SerializedMeshBvh) -> bool {
    a.node_buffer == b.node_buffer
        && a.triangle_order == b.triangle_order
        && a.triangle_indices == b.triangle_indices
        && a.positions == b.positions
}

pub struct BvhcastReference {
    pub pairs: Vec<(usize, usize)>,
    pub a_ids: Vec<usize>,
    pub b_ids: Vec<usize>,
    pub a_neighbors: Vec<usize>,
    pub b_neighbors: Vec<usize>,
    pub shell_bvh: SerializedMeshBvh,
    pub sphere_bvh: SerializedMeshBvh,
}

pub fn intersection_neighbors(map: &super::intersection_map::IntersectionMap) -> Vec<usize> {
    let mut out = Vec::new();
    for &id in &map.ids {
        let neighbors = map
            .intersection_set
            .get(&id)
            .expect("missing neighbor list");
        out.push(neighbors.len());
        out.extend_from_slice(neighbors);
    }
    out
}

fn read_bvh_chunk(raw: &[u8], offset: &mut usize) -> SerializedMeshBvh {
    assert!(raw.len() >= *offset + 20 && &raw[*offset..*offset + 4] == b"BVH1");
    *offset += 4;
    let nb_len = u32::from_le_bytes(raw[*offset..*offset + 4].try_into().unwrap()) as usize;
    *offset += 4;
    let to_len = u32::from_le_bytes(raw[*offset..*offset + 4].try_into().unwrap()) as usize;
    *offset += 4;
    let ti_len = u32::from_le_bytes(raw[*offset..*offset + 4].try_into().unwrap()) as usize;
    *offset += 4;
    let pos_len = u32::from_le_bytes(raw[*offset..*offset + 4].try_into().unwrap()) as usize;
    *offset += 4;

    let read_f32 = |raw: &[u8], offset: &mut usize, n: usize| -> Vec<f32> {
        let bytes = &raw[*offset..*offset + n * 4];
        *offset += n * 4;
        bytes
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes(c.try_into().unwrap()))
            .collect()
    };
    let read_u32 = |raw: &[u8], offset: &mut usize, n: usize| -> Vec<u32> {
        let bytes = &raw[*offset..*offset + n * 4];
        *offset += n * 4;
        bytes
            .chunks_exact(4)
            .map(|c| u32::from_le_bytes(c.try_into().unwrap()))
            .collect()
    };

    SerializedMeshBvh {
        version: 1,
        node_buffer: read_f32(raw, offset, nb_len),
        triangle_order: read_u32(raw, offset, to_len),
        triangle_indices: read_u32(raw, offset, ti_len),
        positions: read_f32(raw, offset, pos_len),
    }
}

pub fn load_bvhcast_reference(raw: &[u8]) -> BvhcastReference {
    assert!(raw.len() >= 16 && &raw[0..4] == b"BCP1");
    let pair_count = u32::from_le_bytes(raw[4..8].try_into().unwrap()) as usize;
    let a_id_count = u32::from_le_bytes(raw[8..12].try_into().unwrap()) as usize;
    let b_id_count = u32::from_le_bytes(raw[12..16].try_into().unwrap()) as usize;
    let (a_neighbor_count, b_neighbor_count, mut offset) = if raw.len() >= 24 {
        (
            u32::from_le_bytes(raw[16..20].try_into().unwrap()) as usize,
            u32::from_le_bytes(raw[20..24].try_into().unwrap()) as usize,
            24usize,
        )
    } else {
        (0usize, 0usize, 16usize)
    };

    let read_u32_vec = |count: usize, raw: &[u8], offset: &mut usize| -> Vec<usize> {
        let vals = raw[*offset..*offset + count * 4]
            .chunks_exact(4)
            .map(|c| u32::from_le_bytes(c.try_into().unwrap()) as usize)
            .collect::<Vec<_>>();
        *offset += count * 4;
        vals
    };

    let flat_pairs = read_u32_vec(pair_count * 2, raw, &mut offset);
    let pairs = flat_pairs.chunks_exact(2).map(|c| (c[0], c[1])).collect();
    let a_ids = read_u32_vec(a_id_count, raw, &mut offset);
    let b_ids = read_u32_vec(b_id_count, raw, &mut offset);
    let a_neighbors = read_u32_vec(a_neighbor_count, raw, &mut offset);
    let b_neighbors = read_u32_vec(b_neighbor_count, raw, &mut offset);
    let shell_bvh = read_bvh_chunk(raw, &mut offset);
    let sphere_bvh = read_bvh_chunk(raw, &mut offset);

    BvhcastReference {
        pairs,
        a_ids,
        b_ids,
        a_neighbors,
        b_neighbors,
        shell_bvh,
        sphere_bvh,
    }
}

pub struct ShellSplitReference {
    pub split_id_count: usize,
    pub tri_keys: Vec<super::topology::TriKey>,
    pub clipped_tri_keys: Vec<super::topology::TriKey>,
    pub clipped_total: usize,
}

pub fn load_shell_split_reference(raw: &[u8]) -> ShellSplitReference {
    use super::topology::TriKey;

    assert!(raw.len() >= 12 && &raw[0..4] == b"SSP1");
    let split_id_count = u32::from_le_bytes(raw[4..8].try_into().unwrap()) as usize;
    let tri_count = u32::from_le_bytes(raw[8..12].try_into().unwrap()) as usize;
    let (clipped_count, clipped_total, mut offset) = if raw.len() >= 20 {
        (
            u32::from_le_bytes(raw[12..16].try_into().unwrap()) as usize,
            u32::from_le_bytes(raw[16..20].try_into().unwrap()) as usize,
            20usize,
        )
    } else {
        (0usize, 0usize, 12usize)
    };

    fn read_keys(raw: &[u8], offset: &mut usize, count: usize) -> Vec<TriKey> {
        let mut tri_keys = Vec::with_capacity(count);
        for _ in 0..count {
            let mut vals = [0i32; 9];
            for v in &mut vals {
                *v = i32::from_le_bytes(raw[*offset..*offset + 4].try_into().unwrap());
                *offset += 4;
            }
            tri_keys.push(TriKey((
                vals[0], vals[1], vals[2], vals[3], vals[4], vals[5], vals[6], vals[7], vals[8],
            )));
        }
        tri_keys
    }

    let tri_keys = read_keys(raw, &mut offset, tri_count);
    let clipped_tri_keys = read_keys(raw, &mut offset, clipped_count);
    ShellSplitReference {
        split_id_count,
        tri_keys,
        clipped_tri_keys,
        clipped_total,
    }
}
