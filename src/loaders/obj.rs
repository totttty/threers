use std::collections::HashMap;

use crate::core::{BufferAttribute, BufferGeometry};

/// Minimal Wavefront OBJ parser. Reads `v` (position), `vn` (normal),
/// `vt` (uv), and `f` (face) records. Negative indices are accepted (offset
/// from end). Quad and polygon faces are triangulated by fan from vertex 0.
///
/// Face-corners that reference the same `v/vt/vn` index triple are **welded**
/// into a single output vertex (indexed), so adjacent faces share vertices.
/// This is what makes `compute_vertex_normals` produce *smooth* shading on
/// normal-less meshes — an unwelded triangle soup can only be flat-shaded.
pub struct ObjLoader;

impl ObjLoader {
    pub fn parse(src: &str) -> BufferGeometry {
        let mut positions: Vec<f32> = Vec::new();
        let mut normals: Vec<f32> = Vec::new();
        let mut uvs: Vec<f32> = Vec::new();

        // Welded per-vertex output, keyed by the (v, vt, vn) index triple.
        let mut out_pos: Vec<f32> = Vec::new();
        let mut out_nrm: Vec<f32> = Vec::new();
        let mut out_uv: Vec<f32> = Vec::new();
        let mut out_idx: Vec<u32> = Vec::new();
        let mut vertex_map: HashMap<(i32, i32, i32), u32> = HashMap::new();

        for raw_line in src.lines() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut parts = line.split_whitespace();
            let Some(tag) = parts.next() else {
                continue;
            };
            match tag {
                "v" => {
                    let xs: Vec<f32> = parts.filter_map(|s| s.parse().ok()).collect();
                    if xs.len() >= 3 {
                        positions.extend_from_slice(&xs[..3]);
                    }
                }
                "vn" => {
                    let xs: Vec<f32> = parts.filter_map(|s| s.parse().ok()).collect();
                    if xs.len() >= 3 {
                        normals.extend_from_slice(&xs[..3]);
                    }
                }
                "vt" => {
                    let xs: Vec<f32> = parts.filter_map(|s| s.parse().ok()).collect();
                    if xs.len() >= 2 {
                        uvs.extend_from_slice(&xs[..2]);
                    }
                }
                "f" => {
                    // Each token is `v[/vt[/vn]]` (1-based, possibly negative).
                    let tokens: Vec<&str> = parts.collect();
                    let mut verts: Vec<(i32, i32, i32)> = Vec::with_capacity(tokens.len());
                    for t in tokens {
                        let pieces: Vec<&str> = t.split('/').collect();
                        let pi: i32 = pieces.first().and_then(|s| s.parse().ok()).unwrap_or(0);
                        let ti: i32 = pieces
                            .get(1)
                            .filter(|s| !s.is_empty())
                            .and_then(|s| s.parse().ok())
                            .unwrap_or(0);
                        let ni: i32 = pieces.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);
                        verts.push((pi, ti, ni));
                    }
                    // Fan-triangulate, welding shared vertices.
                    for i in 1..verts.len().saturating_sub(1) {
                        for v in [verts[0], verts[i], verts[i + 1]] {
                            let index = *vertex_map.entry(v).or_insert_with(|| {
                                let new_index = (out_pos.len() / 3) as u32;
                                push_vertex(
                                    &positions,
                                    &uvs,
                                    &normals,
                                    v,
                                    &mut out_pos,
                                    &mut out_uv,
                                    &mut out_nrm,
                                );
                                new_index
                            });
                            out_idx.push(index);
                        }
                    }
                }
                _ => {}
            }
        }

        let mut g = BufferGeometry::new();
        if !out_pos.is_empty() {
            g.set_attribute("position", BufferAttribute::new(out_pos, 3));
        }
        // Only expose normals the file actually provided (`out_nrm` is always
        // padded with `[0,0,1]` defaults, so guard on the source `vn` records).
        // Downstream can then detect the absence and `compute_vertex_normals`,
        // rather than clobbering an artist's smooth normals.
        if !normals.is_empty() {
            g.set_attribute("normal", BufferAttribute::new(out_nrm, 3));
        }
        if !out_uv.is_empty() {
            g.set_attribute("uv", BufferAttribute::new(out_uv, 2));
        }
        if !out_idx.is_empty() {
            g.set_index(out_idx);
        }
        g
    }
}

fn push_vertex(
    positions: &[f32],
    uvs: &[f32],
    normals: &[f32],
    (pi, ti, ni): (i32, i32, i32),
    out_pos: &mut Vec<f32>,
    out_uv: &mut Vec<f32>,
    out_nrm: &mut Vec<f32>,
) {
    let resolve = |idx: i32, len: usize| -> Option<usize> {
        if idx == 0 {
            return None;
        }
        if idx > 0 {
            Some((idx as usize).saturating_sub(1).min(len.saturating_sub(1)))
        } else {
            let from_end = (-idx) as usize;
            len.checked_sub(from_end)
        }
    };
    if let Some(i) = resolve(pi, positions.len() / 3) {
        out_pos.extend_from_slice(&positions[i * 3..i * 3 + 3]);
    } else {
        out_pos.extend_from_slice(&[0.0, 0.0, 0.0]);
    }
    if let Some(i) = resolve(ti, uvs.len() / 2) {
        out_uv.extend_from_slice(&uvs[i * 2..i * 2 + 2]);
    } else {
        out_uv.extend_from_slice(&[0.0, 0.0]);
    }
    if let Some(i) = resolve(ni, normals.len() / 3) {
        out_nrm.extend_from_slice(&normals[i * 3..i * 3 + 3]);
    } else {
        out_nrm.extend_from_slice(&[0.0, 0.0, 1.0]);
    }
}
