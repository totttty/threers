//! FBX loader. Supports ASCII and binary FBX (Autodesk's node-tree binary
//! format with zlib-compressed property arrays via the DEFLATE codec). Extracts
//! the first `Geometry::Vertices` (position array) + `PolygonVertexIndex`
//! (triangulated index array) into a `BufferGeometry`.

use crate::core::{BufferAttribute, BufferGeometry};

pub struct FbxLoader;

#[derive(Debug)]
pub enum FbxError {
    Unsupported,
    NoGeometry,
    BadHeader,
}

impl FbxLoader {
    pub fn parse(bytes: &[u8]) -> Result<BufferGeometry, FbxError> {
        if bytes.starts_with(b"Kaydara FBX Binary") {
            return parse_binary(bytes);
        }
        let src = std::str::from_utf8(bytes).map_err(|_| FbxError::Unsupported)?;

        let vertices = pull_array(src, "Vertices").ok_or(FbxError::NoGeometry)?;
        let indices = pull_int_array(src, "PolygonVertexIndex").unwrap_or_default();
        Ok(build_geometry(&vertices, &indices))
    }
}

fn build_geometry(vertices: &[f32], indices: &[i32]) -> BufferGeometry {
    let mut positions: Vec<f32> = Vec::new();
    if indices.is_empty() {
        for v in vertices.chunks_exact(3) {
            positions.extend_from_slice(v);
        }
    } else {
        let mut current_polygon: Vec<i32> = Vec::with_capacity(4);
        for &idx in indices {
            let positive = if idx < 0 {
                (!idx) as usize
            } else {
                idx as usize
            };
            current_polygon.push(positive as i32);
            if idx < 0 {
                for i in 1..current_polygon.len().saturating_sub(1) {
                    for &k in &[
                        current_polygon[0],
                        current_polygon[i],
                        current_polygon[i + 1],
                    ] {
                        let k = k as usize;
                        if k * 3 + 2 < vertices.len() {
                            positions.extend_from_slice(&vertices[k * 3..k * 3 + 3]);
                        }
                    }
                }
                current_polygon.clear();
            }
        }
    }
    let mut g = BufferGeometry::new();
    g.set_attribute("position", BufferAttribute::new(positions, 3));
    g
}

// ---- ASCII ----

fn pull_array(src: &str, key: &str) -> Option<Vec<f32>> {
    let pattern = format!("{}: ", key);
    let idx = src.find(&pattern)?;
    let after = &src[idx + pattern.len()..];
    let a_off = after.find("a:")?;
    let after_a = &after[a_off + 2..];
    let close = after_a.find('}').unwrap_or(after_a.len());
    let body = &after_a[..close];
    let nums: Vec<f32> = body
        .split([',', '\n', '\t', ' '].as_ref())
        .filter_map(|t| {
            let t = t.trim();
            if t.is_empty() {
                None
            } else {
                t.parse().ok()
            }
        })
        .collect();
    Some(nums)
}

fn pull_int_array(src: &str, key: &str) -> Option<Vec<i32>> {
    let pattern = format!("{}: ", key);
    let idx = src.find(&pattern)?;
    let after = &src[idx + pattern.len()..];
    let a_off = after.find("a:")?;
    let after_a = &after[a_off + 2..];
    let close = after_a.find('}').unwrap_or(after_a.len());
    let body = &after_a[..close];
    let nums: Vec<i32> = body
        .split([',', '\n', '\t', ' '].as_ref())
        .filter_map(|t| {
            let t = t.trim();
            if t.is_empty() {
                None
            } else {
                t.parse().ok()
            }
        })
        .collect();
    Some(nums)
}

// ---- Binary ----

fn parse_binary(bytes: &[u8]) -> Result<BufferGeometry, FbxError> {
    if bytes.len() < 27 {
        return Err(FbxError::BadHeader);
    }
    // Header: 21 bytes "Kaydara FBX Binary  " + magic [0x1A, 0x00] + version u32 LE.
    let version = u32::from_le_bytes([bytes[23], bytes[24], bytes[25], bytes[26]]);
    let use_64bit_offsets = version >= 7500;
    let mut p = 27usize;
    let mut found_vertices: Option<Vec<f32>> = None;
    let mut found_indices: Option<Vec<i32>> = None;
    walk_nodes(
        bytes,
        &mut p,
        use_64bit_offsets,
        &mut found_vertices,
        &mut found_indices,
    );
    let v = found_vertices.ok_or(FbxError::NoGeometry)?;
    let i = found_indices.unwrap_or_default();
    Ok(build_geometry(&v, &i))
}

fn walk_nodes(
    bytes: &[u8],
    p: &mut usize,
    use_64: bool,
    found_v: &mut Option<Vec<f32>>,
    found_i: &mut Option<Vec<i32>>,
) {
    loop {
        let header_size = if use_64 { 25 } else { 13 };
        if *p + header_size > bytes.len() {
            return;
        }
        let (end_offset, num_props, _prop_list_len, name_len) = if use_64 {
            let eo = u64::from_le_bytes([
                bytes[*p],
                bytes[*p + 1],
                bytes[*p + 2],
                bytes[*p + 3],
                bytes[*p + 4],
                bytes[*p + 5],
                bytes[*p + 6],
                bytes[*p + 7],
            ]) as usize;
            let np = u64::from_le_bytes([
                bytes[*p + 8],
                bytes[*p + 9],
                bytes[*p + 10],
                bytes[*p + 11],
                bytes[*p + 12],
                bytes[*p + 13],
                bytes[*p + 14],
                bytes[*p + 15],
            ]) as usize;
            let pll = u64::from_le_bytes([
                bytes[*p + 16],
                bytes[*p + 17],
                bytes[*p + 18],
                bytes[*p + 19],
                bytes[*p + 20],
                bytes[*p + 21],
                bytes[*p + 22],
                bytes[*p + 23],
            ]) as usize;
            let nl = bytes[*p + 24] as usize;
            (eo, np, pll, nl)
        } else {
            let eo = u32::from_le_bytes([bytes[*p], bytes[*p + 1], bytes[*p + 2], bytes[*p + 3]])
                as usize;
            let np =
                u32::from_le_bytes([bytes[*p + 4], bytes[*p + 5], bytes[*p + 6], bytes[*p + 7]])
                    as usize;
            let pll =
                u32::from_le_bytes([bytes[*p + 8], bytes[*p + 9], bytes[*p + 10], bytes[*p + 11]])
                    as usize;
            let nl = bytes[*p + 12] as usize;
            (eo, np, pll, nl)
        };
        if end_offset == 0 {
            *p += header_size;
            return;
        }
        *p += header_size;
        if *p + name_len > bytes.len() {
            return;
        }
        let name = String::from_utf8_lossy(&bytes[*p..*p + name_len]).to_string();
        *p += name_len;
        let mut props: Vec<FbxProperty> = Vec::with_capacity(num_props);
        for _ in 0..num_props {
            if *p >= bytes.len() {
                return;
            }
            let ty = bytes[*p];
            *p += 1;
            let prop = read_property(bytes, p, ty);
            props.push(prop);
        }
        if name == "Vertices" && found_v.is_none() {
            if let Some(FbxProperty::DoubleArray(d)) = props.first() {
                *found_v = Some(d.iter().map(|v| *v as f32).collect());
            } else if let Some(FbxProperty::FloatArray(f)) = props.first() {
                *found_v = Some(f.clone());
            }
        }
        if name == "PolygonVertexIndex" && found_i.is_none() {
            if let Some(FbxProperty::IntArray(i)) = props.first() {
                *found_i = Some(i.clone());
            } else if let Some(FbxProperty::LongArray(l)) = props.first() {
                *found_i = Some(l.iter().map(|v| *v as i32).collect());
            }
        }
        if *p < end_offset {
            walk_nodes(bytes, p, use_64, found_v, found_i);
        }
        if *p < end_offset {
            *p = end_offset;
        }
    }
}

#[derive(Debug)]
enum FbxProperty {
    Bool(#[allow(dead_code)] bool),
    Short(#[allow(dead_code)] i16),
    Int(#[allow(dead_code)] i32),
    Long(#[allow(dead_code)] i64),
    Float(#[allow(dead_code)] f32),
    Double(#[allow(dead_code)] f64),
    String(#[allow(dead_code)] String),
    Raw(#[allow(dead_code)] Vec<u8>),
    FloatArray(Vec<f32>),
    DoubleArray(Vec<f64>),
    IntArray(Vec<i32>),
    LongArray(Vec<i64>),
    BoolArray(#[allow(dead_code)] Vec<bool>),
}

fn read_property(bytes: &[u8], p: &mut usize, ty: u8) -> FbxProperty {
    let read_u32 = |bytes: &[u8], p: &mut usize| -> u32 {
        let v = u32::from_le_bytes([bytes[*p], bytes[*p + 1], bytes[*p + 2], bytes[*p + 3]]);
        *p += 4;
        v
    };
    match ty {
        b'C' => {
            let v = bytes[*p] != 0;
            *p += 1;
            FbxProperty::Bool(v)
        }
        b'Y' => {
            let v = i16::from_le_bytes([bytes[*p], bytes[*p + 1]]);
            *p += 2;
            FbxProperty::Short(v)
        }
        b'I' => {
            let v = i32::from_le_bytes([bytes[*p], bytes[*p + 1], bytes[*p + 2], bytes[*p + 3]]);
            *p += 4;
            FbxProperty::Int(v)
        }
        b'L' => {
            let v = i64::from_le_bytes([
                bytes[*p],
                bytes[*p + 1],
                bytes[*p + 2],
                bytes[*p + 3],
                bytes[*p + 4],
                bytes[*p + 5],
                bytes[*p + 6],
                bytes[*p + 7],
            ]);
            *p += 8;
            FbxProperty::Long(v)
        }
        b'F' => {
            let v = f32::from_le_bytes([bytes[*p], bytes[*p + 1], bytes[*p + 2], bytes[*p + 3]]);
            *p += 4;
            FbxProperty::Float(v)
        }
        b'D' => {
            let v = f64::from_le_bytes([
                bytes[*p],
                bytes[*p + 1],
                bytes[*p + 2],
                bytes[*p + 3],
                bytes[*p + 4],
                bytes[*p + 5],
                bytes[*p + 6],
                bytes[*p + 7],
            ]);
            *p += 8;
            FbxProperty::Double(v)
        }
        b'S' | b'R' => {
            let len = read_u32(bytes, p) as usize;
            let data = bytes[*p..*p + len].to_vec();
            *p += len;
            if ty == b'S' {
                FbxProperty::String(String::from_utf8_lossy(&data).to_string())
            } else {
                FbxProperty::Raw(data)
            }
        }
        b'd' | b'f' | b'l' | b'i' | b'b' => {
            let count = read_u32(bytes, p) as usize;
            let encoding = read_u32(bytes, p);
            let comp_len = read_u32(bytes, p) as usize;
            let data: Vec<u8> = if encoding == 0 {
                let n = match ty {
                    b'd' => 8,
                    b'f' => 4,
                    b'l' => 8,
                    b'i' => 4,
                    b'b' => 1,
                    _ => 4,
                } * count;
                let d = bytes[*p..*p + n].to_vec();
                *p += n;
                d
            } else {
                let raw = &bytes[*p..*p + comp_len];
                *p += comp_len;
                super::deflate::inflate_zlib(raw).unwrap_or_default()
            };
            match ty {
                b'd' => {
                    let vs: Vec<f64> = data
                        .chunks_exact(8)
                        .map(|c| {
                            f64::from_le_bytes([c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]])
                        })
                        .collect();
                    FbxProperty::DoubleArray(vs)
                }
                b'f' => {
                    let vs: Vec<f32> = data
                        .chunks_exact(4)
                        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                        .collect();
                    FbxProperty::FloatArray(vs)
                }
                b'l' => {
                    let vs: Vec<i64> = data
                        .chunks_exact(8)
                        .map(|c| {
                            i64::from_le_bytes([c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]])
                        })
                        .collect();
                    FbxProperty::LongArray(vs)
                }
                b'i' => {
                    let vs: Vec<i32> = data
                        .chunks_exact(4)
                        .map(|c| i32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                        .collect();
                    FbxProperty::IntArray(vs)
                }
                _ => FbxProperty::BoolArray(data.iter().map(|&b| b != 0).collect()),
            }
        }
        _ => FbxProperty::Int(0),
    }
}
