//! OpenEXR loader. Parses the EXR header (magic + version + attribute table),
//! decodes uncompressed scanlines (Compression::NONE) for RGB/RGBA HALF or
//! FLOAT channels into 8-bit-clipped RGBA. Compressed codecs (ZIP/PIZ/B44/
//! DWA) return `Unsupported`.

use crate::textures::{Texture, TextureFormat};

pub struct ExrLoader;

#[derive(Debug)]
pub enum ExrError {
    BadMagic,
    BadHeader,
    /// Generic fallback for `UnsupportedPiz` / `UnsupportedB44` / `UnsupportedDwa`
    /// when downstream code doesn't need the specific tag.
    UnsupportedCompression,
    /// PIZ wavelet + Huffman. Complex; needs ~1k LOC port. Not implemented.
    UnsupportedPiz,
    /// B44/B44A 16×16 fixed-rate block encoding. Not implemented.
    UnsupportedB44,
    /// DWA (DreamWorks) JPEG-like codec. Not implemented.
    UnsupportedDwa,
    UnsupportedChannel,
}

const MAGIC: [u8; 4] = [0x76, 0x2f, 0x31, 0x01];

impl ExrLoader {
    pub fn parse(bytes: &[u8]) -> Result<Texture, ExrError> {
        if bytes.len() < 8 || bytes[0..4] != MAGIC {
            return Err(ExrError::BadMagic);
        }
        // version: u32 LE — low byte version, high 24 bits = flags.
        let mut pos = 8usize;
        // Attribute table: triples of (name\0, type\0, size: u32 LE, data...)
        // until a single null byte.
        let mut data_window: Option<[i32; 4]> = None;
        let mut compression: u8 = 0; // 0 == NONE
        let mut channels_present: Vec<(String, u8)> = Vec::new(); // (name, pixel_type)
        loop {
            if pos >= bytes.len() {
                return Err(ExrError::BadHeader);
            }
            if bytes[pos] == 0 {
                pos += 1;
                break;
            }
            // Read attribute name (zero-terminated).
            let name_start = pos;
            while pos < bytes.len() && bytes[pos] != 0 {
                pos += 1;
            }
            if pos >= bytes.len() {
                return Err(ExrError::BadHeader);
            }
            let name = String::from_utf8_lossy(&bytes[name_start..pos]).to_string();
            pos += 1;
            // Type.
            let type_start = pos;
            while pos < bytes.len() && bytes[pos] != 0 {
                pos += 1;
            }
            if pos >= bytes.len() {
                return Err(ExrError::BadHeader);
            }
            let attr_type = String::from_utf8_lossy(&bytes[type_start..pos]).to_string();
            pos += 1;
            // Size.
            if pos + 4 > bytes.len() {
                return Err(ExrError::BadHeader);
            }
            let size =
                u32::from_le_bytes([bytes[pos], bytes[pos + 1], bytes[pos + 2], bytes[pos + 3]])
                    as usize;
            pos += 4;
            if pos + size > bytes.len() {
                return Err(ExrError::BadHeader);
            }
            let data = &bytes[pos..pos + size];
            pos += size;
            match name.as_str() {
                "dataWindow" if attr_type == "box2i" && size >= 16 => {
                    let read_i =
                        |o| i32::from_le_bytes([data[o], data[o + 1], data[o + 2], data[o + 3]]);
                    data_window = Some([read_i(0), read_i(4), read_i(8), read_i(12)]);
                }
                "compression" if attr_type == "compression" && size >= 1 => {
                    compression = data[0];
                }
                "channels" if attr_type == "chlist" => {
                    let mut k = 0usize;
                    while k < size {
                        if data[k] == 0 {
                            break;
                        }
                        let n_start = k;
                        while k < size && data[k] != 0 {
                            k += 1;
                        }
                        let cname = String::from_utf8_lossy(&data[n_start..k]).to_string();
                        k += 1;
                        if k + 16 > size {
                            break;
                        }
                        let pixel_type = data[k];
                        k += 16;
                        channels_present.push((cname, pixel_type));
                    }
                }
                _ => {}
            }
        }
        // 0 = NONE, 1 = RLE (TODO), 2 = ZIPS, 3 = ZIP, 4 = PIZ,
        // 5 = PXR24, 6 = B44, 7 = B44A, 8 = DWAA, 9 = DWAB.
        match compression {
            0 | 2 | 3 => {}
            4 => return Err(ExrError::UnsupportedPiz),
            6 | 7 => return Err(ExrError::UnsupportedB44),
            8 | 9 => return Err(ExrError::UnsupportedDwa),
            _ => return Err(ExrError::UnsupportedCompression),
        }
        let Some(dw) = data_window else {
            return Err(ExrError::BadHeader);
        };
        let width = (dw[2] - dw[0] + 1).max(0) as usize;
        let height = (dw[3] - dw[1] + 1).max(0) as usize;
        if width == 0 || height == 0 {
            return Err(ExrError::BadHeader);
        }

        // Determine RGB(A) channel layout.
        let pixel_type_of = |name: &str| -> Option<u8> {
            channels_present
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, t)| *t)
        };
        let r_type = pixel_type_of("R").ok_or(ExrError::UnsupportedChannel)?;
        let g_type = pixel_type_of("G").ok_or(ExrError::UnsupportedChannel)?;
        let b_type = pixel_type_of("B").ok_or(ExrError::UnsupportedChannel)?;
        let a_type = pixel_type_of("A");

        // Skip the scanline-offsets table: height * u64.
        let offsets_size = height * 8;
        if pos + offsets_size > bytes.len() {
            return Err(ExrError::BadHeader);
        }
        pos += offsets_size;

        // For each scanline: y(i32) + size(u32) + interleaved channel data.
        let sample_size = |t: u8| -> usize {
            match t {
                1 => 2,
                2 => 4,
                _ => 0,
            }
        };
        let read_sample = |t: u8, data: &[u8], off: usize| -> f32 {
            match t {
                1 => f16_to_f32(u16::from_le_bytes([data[off], data[off + 1]])),
                2 => f32::from_le_bytes([data[off], data[off + 1], data[off + 2], data[off + 3]]),
                _ => 0.0,
            }
        };

        let mut rgba: Vec<u8> = Vec::with_capacity(width * height * 4);
        let scanlines_per_block = if compression == 3 { 16 } else { 1 };
        let mut y = 0usize;
        while y < height {
            if pos + 8 > bytes.len() {
                return Err(ExrError::BadHeader);
            }
            let _scanline_y =
                i32::from_le_bytes([bytes[pos], bytes[pos + 1], bytes[pos + 2], bytes[pos + 3]]);
            let block_size = u32::from_le_bytes([
                bytes[pos + 4],
                bytes[pos + 5],
                bytes[pos + 6],
                bytes[pos + 7],
            ]) as usize;
            pos += 8;
            if pos + block_size > bytes.len() {
                return Err(ExrError::BadHeader);
            }
            let block_compressed = &bytes[pos..pos + block_size];
            pos += block_size;
            // Decompress if needed.
            let block: Vec<u8> = if compression == 0 {
                block_compressed.to_vec()
            } else {
                match super::deflate::inflate_zlib(block_compressed) {
                    Ok(v) => exr_reorder(&v),
                    Err(_) => return Err(ExrError::UnsupportedCompression),
                }
            };
            // For each scanline in the block, parse channels.
            let scanline_count = scanlines_per_block.min(height - y);
            let mut sorted_chans: Vec<&(String, u8)> = channels_present.iter().collect();
            sorted_chans.sort_by(|a, b| a.0.cmp(&b.0));
            let bytes_per_scanline: usize = sorted_chans
                .iter()
                .map(|(_, t)| sample_size(*t) * width)
                .sum();
            for s in 0..scanline_count {
                if (s + 1) * bytes_per_scanline > block.len() {
                    break;
                }
                let scanline = &block[s * bytes_per_scanline..(s + 1) * bytes_per_scanline];

                let mut channel_offsets: std::collections::HashMap<&str, usize> =
                    std::collections::HashMap::new();
                let mut cursor = 0usize;
                for (name, t) in &sorted_chans {
                    channel_offsets.insert(name.as_str(), cursor);
                    cursor += sample_size(*t) * width;
                }
                for x in 0..width {
                    let r = read_sample(
                        r_type,
                        scanline,
                        channel_offsets["R"] + x * sample_size(r_type),
                    );
                    let g = read_sample(
                        g_type,
                        scanline,
                        channel_offsets["G"] + x * sample_size(g_type),
                    );
                    let b = read_sample(
                        b_type,
                        scanline,
                        channel_offsets["B"] + x * sample_size(b_type),
                    );
                    let a = if let Some(t) = a_type {
                        read_sample(t, scanline, channel_offsets["A"] + x * sample_size(t))
                    } else {
                        1.0
                    };
                    let clip = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
                    rgba.extend_from_slice(&[clip(r), clip(g), clip(b), clip(a)]);
                }
            }
            y += scanline_count;
        }

        Ok(Texture::new(
            width as u32,
            height as u32,
            TextureFormat::Rgba8Unorm,
            rgba,
        ))
    }
}

/// EXR ZIP reorder step: 1) reverse the up-delta predictor (each byte adds the
/// previous to recover the original) 2) un-interleave odd/even halves.
fn exr_reorder(input: &[u8]) -> Vec<u8> {
    if input.is_empty() {
        return Vec::new();
    }
    // Un-interleave: first the bytes at even positions, then odd.
    let half = (input.len() + 1) / 2;
    let mut out = vec![0u8; input.len()];
    for (i, &b) in input.iter().enumerate() {
        let dst = if i < half { i * 2 } else { (i - half) * 2 + 1 };
        if dst < out.len() {
            out[dst] = b;
        }
    }
    // Reverse delta encoding.
    for i in 1..out.len() {
        out[i] = out[i].wrapping_add(out[i - 1]).wrapping_sub(128);
    }
    // First byte fix-up.
    if !out.is_empty() {
        out[0] = out[0].wrapping_sub(128);
    }
    out
}

/// IEEE 754 half-precision → single-precision conversion.
fn f16_to_f32(h: u16) -> f32 {
    let sign = ((h >> 15) & 1) as u32;
    let exp = ((h >> 10) & 0x1f) as u32;
    let mant = (h & 0x3ff) as u32;
    let bits = match exp {
        0 if mant == 0 => sign << 31,
        0 => {
            // Subnormal.
            let m = mant as f32;
            let v = (m / 1024.0) * 2f32.powi(-14);
            return if sign == 0 { v } else { -v };
        }
        31 => (sign << 31) | (0xff << 23) | (mant << 13),
        _ => (sign << 31) | ((exp + 112) << 23) | (mant << 13),
    };
    f32::from_bits(bits)
}
