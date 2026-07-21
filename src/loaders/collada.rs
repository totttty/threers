use super::xml;
use crate::core::{BufferAttribute, BufferGeometry};

pub struct ColladaLoader;

#[derive(Debug)]
pub enum ColladaError {
    Xml(&'static str),
    NoGeometry,
    Unsupported,
}

impl ColladaLoader {
    /// Parse a Collada (.dae) document and return the first geometry's
    /// triangulated `BufferGeometry`.
    pub fn parse(src: &str) -> Result<BufferGeometry, ColladaError> {
        let root = xml::parse(src).map_err(|_| ColladaError::Xml("xml parse"))?;
        let lib_geom = root
            .find_child("library_geometries")
            .ok_or(ColladaError::NoGeometry)?;
        let geom = lib_geom
            .find_child("geometry")
            .ok_or(ColladaError::NoGeometry)?;
        let mesh = geom.find_child("mesh").ok_or(ColladaError::NoGeometry)?;

        let mut sources: std::collections::HashMap<String, Vec<f32>> =
            std::collections::HashMap::new();
        for src_elem in mesh.find_children("source") {
            let id = src_elem.attributes.get("id").cloned().unwrap_or_default();
            if let Some(fa) = src_elem.find_child("float_array") {
                let nums: Vec<f32> = fa
                    .text
                    .split_whitespace()
                    .filter_map(|s| s.parse().ok())
                    .collect();
                sources.insert(id, nums);
            }
        }

        let tri = mesh.find_child("triangles");
        let polylist = mesh.find_child("polylist");
        let prim = match (tri, polylist) {
            (Some(t), _) => t,
            (None, Some(p)) => p,
            _ => return Err(ColladaError::NoGeometry),
        };

        struct Input {
            semantic: String,
            source: String,
            offset: usize,
        }
        let mut inputs: Vec<Input> = Vec::new();
        for inp in prim.find_children("input") {
            let semantic = inp.attributes.get("semantic").cloned().unwrap_or_default();
            let source = inp
                .attributes
                .get("source")
                .cloned()
                .unwrap_or_default()
                .trim_start_matches('#')
                .to_string();
            let offset: usize = inp
                .attributes
                .get("offset")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            inputs.push(Input {
                semantic,
                source,
                offset,
            });
        }
        let stride = inputs.iter().map(|i| i.offset).max().unwrap_or(0) + 1;

        let vertex_source: Option<String> = (|| -> Option<String> {
            let v = mesh.find_child("vertices")?;
            for inp in v.find_children("input") {
                if inp
                    .attributes
                    .get("semantic")
                    .map(|s| s == "POSITION")
                    .unwrap_or(false)
                {
                    return inp
                        .attributes
                        .get("source")
                        .map(|s| s.trim_start_matches('#').to_string());
                }
            }
            None
        })();

        let p_elem = prim.find_child("p").ok_or(ColladaError::NoGeometry)?;
        let indices: Vec<u32> = p_elem
            .text
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();
        if indices.is_empty() {
            return Err(ColladaError::NoGeometry);
        }

        let resolve = |sem: &str| -> Option<&Vec<f32>> {
            let id = inputs.iter().find(|i| i.semantic == sem)?;
            if id.semantic == "VERTEX" {
                vertex_source.as_ref().and_then(|s| sources.get(s))
            } else {
                sources.get(&id.source)
            }
        };

        let pos_arr = resolve("VERTEX")
            .or_else(|| resolve("POSITION"))
            .ok_or(ColladaError::NoGeometry)?;
        let nrm_arr = resolve("NORMAL");
        let uv_arr = resolve("TEXCOORD");

        let pos_off = inputs
            .iter()
            .find(|i| i.semantic == "VERTEX" || i.semantic == "POSITION")
            .map(|i| i.offset)
            .unwrap_or(0);
        let nrm_off = inputs
            .iter()
            .find(|i| i.semantic == "NORMAL")
            .map(|i| i.offset);
        let uv_off = inputs
            .iter()
            .find(|i| i.semantic == "TEXCOORD")
            .map(|i| i.offset);

        let mut positions: Vec<f32> = Vec::new();
        let mut normals: Vec<f32> = Vec::new();
        let mut uvs: Vec<f32> = Vec::new();
        let tri_count = indices.len() / (stride * 3);
        for t in 0..tri_count {
            for v in 0..3 {
                let base = (t * 3 + v) * stride;
                let pi = indices[base + pos_off] as usize;
                positions.extend_from_slice(&pos_arr[pi * 3..pi * 3 + 3]);
                if let (Some(off), Some(arr)) = (nrm_off, nrm_arr) {
                    let ni = indices[base + off] as usize;
                    normals.extend_from_slice(&arr[ni * 3..ni * 3 + 3]);
                }
                if let (Some(off), Some(arr)) = (uv_off, uv_arr) {
                    let ui = indices[base + off] as usize;
                    uvs.extend_from_slice(&arr[ui * 2..ui * 2 + 2]);
                }
            }
        }

        let mut g = BufferGeometry::new();
        g.set_attribute("position", BufferAttribute::new(positions, 3));
        if !normals.is_empty() {
            g.set_attribute("normal", BufferAttribute::new(normals, 3));
        }
        if !uvs.is_empty() {
            g.set_attribute("uv", BufferAttribute::new(uvs, 2));
        }
        Ok(g)
    }
}
