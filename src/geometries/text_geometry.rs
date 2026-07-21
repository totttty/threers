use super::ExtrudeGeometry;
use crate::core::{BufferAttribute, BufferGeometry};
use crate::curves::Shape;
use crate::math::Vector3;

pub struct TextGeometry;

/// One glyph's outline + its post-glyph advance.
pub struct Glyph {
    pub shape: Shape,
    /// X advance after this glyph (next glyph's origin offset).
    pub advance: f32,
}

impl TextGeometry {
    /// Lay out a sequence of caller-supplied glyph shapes left-to-right and
    /// extrude them to `depth`. Mirrors three.js's `TextGeometry`. Font
    /// parsing stays out of the crate — the caller (or a downstream FontLoader)
    /// is responsible for producing `Shape`s from glyph outlines.
    pub fn new(glyphs: &[Glyph], depth: f32, curve_segments: usize) -> BufferGeometry {
        let mut positions: Vec<f32> = Vec::new();
        let mut normals: Vec<f32> = Vec::new();
        let mut uvs: Vec<f32> = Vec::new();
        let mut indices: Vec<u32> = Vec::new();
        let mut x_offset = 0.0_f32;

        for g in glyphs {
            let glyph_geom = ExtrudeGeometry::new(&g.shape, depth, curve_segments);
            let base_vert = (positions.len() / 3) as u32;
            if let Some(p) = glyph_geom.get_attribute("position") {
                // Translate every position by (x_offset, 0, 0).
                for chunk in p.array.chunks_exact(3) {
                    positions.extend_from_slice(&[chunk[0] + x_offset, chunk[1], chunk[2]]);
                }
            }
            if let Some(n) = glyph_geom.get_attribute("normal") {
                normals.extend_from_slice(&n.array);
            }
            if let Some(u) = glyph_geom.get_attribute("uv") {
                uvs.extend_from_slice(&u.array);
            }
            if let Some(idx) = &glyph_geom.index {
                for i in idx {
                    indices.push(i + base_vert);
                }
            }
            x_offset += g.advance;
        }

        let mut out = BufferGeometry::new();
        out.set_attribute("position", BufferAttribute::new(positions, 3));
        if !normals.is_empty() {
            out.set_attribute("normal", BufferAttribute::new(normals, 3));
        }
        if !uvs.is_empty() {
            out.set_attribute("uv", BufferAttribute::new(uvs, 2));
        }
        if !indices.is_empty() {
            out.set_index(indices);
        }
        let _ = Vector3::ZERO; // silence unused-import lint
        out
    }
}
