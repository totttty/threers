use crate::textures::{Texture, TextureFormat};

/// Radiance .hdr (RGBE) loader. Parses the ASCII header, then RLE-decompressed
/// RGBE scanlines into linear-RGB float values (stored here as Rgba8Unorm by
/// tonemap-clipping into 0–255). For a true HDR pipeline, swap to Rgba16Float
/// once the renderer exposes a half-float texture format.
pub struct HdrLoader;

#[derive(Debug)]
pub enum HdrError {
    BadMagic,
    BadHeader,
    BadScanline,
}

impl HdrLoader {
    pub fn parse(bytes: &[u8]) -> Result<Texture, HdrError> {
        // Header is ASCII terminated by an empty line, then a dimension line
        // like "-Y H +X W", then binary RLE-RGBE scanlines.
        let mut pos = 0;
        if !bytes.starts_with(b"#?RADIANCE") && !bytes.starts_with(b"#?RGBE") {
            return Err(HdrError::BadMagic);
        }
        // Skip past first newline.
        while pos < bytes.len() && bytes[pos] != b'\n' {
            pos += 1;
        }
        pos += 1;
        // Read header lines until empty line.
        loop {
            let line_start = pos;
            while pos < bytes.len() && bytes[pos] != b'\n' {
                pos += 1;
            }
            let line = &bytes[line_start..pos];
            pos += 1;
            if line.is_empty() {
                break;
            }
        }
        // Dimension line.
        let dim_start = pos;
        while pos < bytes.len() && bytes[pos] != b'\n' {
            pos += 1;
        }
        let dim_line =
            std::str::from_utf8(&bytes[dim_start..pos]).map_err(|_| HdrError::BadHeader)?;
        pos += 1;
        let mut height = 0usize;
        let mut width = 0usize;
        for tok in dim_line.split_whitespace() {
            if let Ok(n) = tok.parse::<usize>() {
                if height == 0 {
                    height = n;
                } else {
                    width = n;
                }
            }
        }
        if width == 0 || height == 0 {
            return Err(HdrError::BadHeader);
        }

        // Decode scanlines. Each scanline is `width` pixels of RGBE.
        let mut rgba = Vec::with_capacity(width * height * 4);
        for _ in 0..height {
            if pos + 4 > bytes.len() {
                return Err(HdrError::BadScanline);
            }
            let r = bytes[pos];
            let g = bytes[pos + 1];
            let b1 = bytes[pos + 2];
            let b2 = bytes[pos + 3];
            // Detect new-RLE scanline: `2,2,(width>>8),(width&0xff)` (width <= 32767).
            if r == 2 && g == 2 && b1 < 128 {
                let scan_w = ((b1 as usize) << 8) | b2 as usize;
                pos += 4;
                if scan_w != width {
                    return Err(HdrError::BadScanline);
                }
                let mut channels: [Vec<u8>; 4] = [
                    vec![0u8; width],
                    vec![0u8; width],
                    vec![0u8; width],
                    vec![0u8; width],
                ];
                for c in &mut channels {
                    let mut x = 0usize;
                    while x < width {
                        if pos >= bytes.len() {
                            return Err(HdrError::BadScanline);
                        }
                        let n = bytes[pos];
                        pos += 1;
                        if n > 128 {
                            // Run-length
                            let run = (n - 128) as usize;
                            if pos >= bytes.len() {
                                return Err(HdrError::BadScanline);
                            }
                            let val = bytes[pos];
                            pos += 1;
                            for _ in 0..run {
                                c[x] = val;
                                x += 1;
                            }
                        } else {
                            // Literal
                            let run = n as usize;
                            for _ in 0..run {
                                if pos >= bytes.len() {
                                    return Err(HdrError::BadScanline);
                                }
                                c[x] = bytes[pos];
                                pos += 1;
                                x += 1;
                            }
                        }
                    }
                }
                for x in 0..width {
                    let rgb = rgbe_to_rgb(
                        channels[0][x],
                        channels[1][x],
                        channels[2][x],
                        channels[3][x],
                    );
                    rgba.extend_from_slice(&rgb);
                }
            } else {
                // Old-style flat RGBE scanline (no RLE).
                pos += 4;
                let rgb = rgbe_to_rgb(r, g, b1, b2);
                rgba.extend_from_slice(&rgb);
                for _ in 1..width {
                    if pos + 4 > bytes.len() {
                        return Err(HdrError::BadScanline);
                    }
                    let rgb =
                        rgbe_to_rgb(bytes[pos], bytes[pos + 1], bytes[pos + 2], bytes[pos + 3]);
                    rgba.extend_from_slice(&rgb);
                    pos += 4;
                }
            }
        }

        Ok(Texture::new(
            width as u32,
            height as u32,
            TextureFormat::Rgba8Unorm,
            rgba,
        ))
    }
}

fn rgbe_to_rgb(r: u8, g: u8, b: u8, e: u8) -> [u8; 4] {
    if e == 0 {
        return [0, 0, 0, 255];
    }
    let f = (2.0_f32).powi(e as i32 - 128 - 8);
    let rf = r as f32 * f;
    let gf = g as f32 * f;
    let bf = b as f32 * f;
    let clip = |x: f32| (x.clamp(0.0, 1.0) * 255.0).round() as u8;
    [clip(rf), clip(gf), clip(bf), 255]
}
