#![allow(dead_code)]

use crate::textures::{
    CubeTexture, CubeUvAtlas, Texture, TextureFilter, TextureFormat, TextureWrap,
};
use std::sync::Arc;

/// WebGPU `write_texture` requires `bytes_per_row` to be a multiple of 256 when height > 1.
const TEXTURE_ROW_ALIGN: u32 = 256;

fn pad_rows_for_upload(pixels: &[u8], width: u32, height: u32, bpp: u32) -> (Vec<u8>, u32) {
    let unpadded = width * bpp;
    let padded = unpadded.div_ceil(TEXTURE_ROW_ALIGN) * TEXTURE_ROW_ALIGN;
    if padded == unpadded {
        return (pixels.to_vec(), padded);
    }
    let mut out = vec![0u8; (padded * height) as usize];
    for y in 0..height as usize {
        let src = y * unpadded as usize;
        let dst = y * padded as usize;
        out[dst..dst + unpadded as usize].copy_from_slice(&pixels[src..src + unpadded as usize]);
    }
    (out, padded)
}

/// IEEE-754 f32 → f16 (PMREM CubeUV atlas matches three.js HalfFloat).
pub(crate) fn f32_to_f16_bits(v: f32) -> u16 {
    let bits = v.to_bits();
    let sign = ((bits >> 16) & 0x8000) as u16;
    let exponent = ((bits >> 23) & 0xff) as i32;
    let mantissa = bits & 0x007f_ffff;

    if exponent == 0xff {
        return if mantissa == 0 {
            sign | 0x7c00
        } else {
            sign | 0x7e00
        };
    }

    let mut half_exponent = exponent - 127 + 15;
    if half_exponent >= 31 {
        // Preserve the previous finite-overflow contract: saturate instead of
        // introducing infinities into HDR textures.
        return sign | 0x7bff;
    }
    if half_exponent <= 0 {
        if half_exponent < -10 {
            return sign;
        }
        let significand = mantissa | 0x0080_0000;
        let shift = (14 - half_exponent) as u32;
        let mut half_mantissa = significand >> shift;
        let remainder = significand & ((1u32 << shift) - 1);
        let halfway = 1u32 << (shift - 1);
        if remainder > halfway || (remainder == halfway && half_mantissa & 1 != 0) {
            half_mantissa += 1;
        }
        return sign | half_mantissa as u16;
    }

    let mut half_mantissa = mantissa >> 13;
    let remainder = mantissa & 0x1fff;
    if remainder > 0x1000 || (remainder == 0x1000 && half_mantissa & 1 != 0) {
        half_mantissa += 1;
        if half_mantissa == 0x400 {
            half_mantissa = 0;
            half_exponent += 1;
            if half_exponent >= 31 {
                return sign | 0x7bff;
            }
        }
    }
    sign | ((half_exponent as u16) << 10) | half_mantissa as u16
}

pub(crate) fn f16_bits_to_f32(h: u16) -> f32 {
    let s = ((h & 0x8000) as u32) << 16;
    let e = ((h >> 10) & 0x1f) as u32;
    let m = (h & 0x3ff) as u32;
    if e == 0 {
        if m == 0 {
            return f32::from_bits(s);
        }
        let mut e = 1u32;
        let mut m = m;
        while (m & 0x400) == 0 {
            m <<= 1;
            e += 1;
        }
        m &= 0x3ff;
        return f32::from_bits(s | ((127 - 15 - e) << 23) | (m << 13));
    }
    if e == 31 {
        return f32::from_bits(s | 0x7f80_0000 | (m << 13));
    }
    f32::from_bits(s | ((e + 127 - 15) << 23) | (m << 13))
}

fn pad_rows_rgba16f(f32_pixels: &[f32], width: u32, height: u32) -> (Vec<u8>, u32) {
    let bpp = 8u32;
    let unpadded = width * bpp;
    let padded = unpadded.div_ceil(TEXTURE_ROW_ALIGN) * TEXTURE_ROW_ALIGN;
    let mut out = vec![0u8; (padded * height) as usize];
    for y in 0..height {
        for x in 0..width {
            let fi = ((y * width + x) * 4) as usize;
            let dst = (y * padded + x * bpp) as usize;
            for c in 0..4usize {
                let h = f32_to_f16_bits(f32_pixels[fi + c]);
                out[dst + c * 2..dst + c * 2 + 2].copy_from_slice(&h.to_le_bytes());
            }
        }
    }
    (out, padded)
}

/// GPU-side handle for a 2D texture.
pub struct GpuTexture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
}

/// GPU-side handle for a cubemap (six faces) and optional CubeUV 2D atlas.
pub struct GpuCubeTexture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub cube_uv_texture: Option<wgpu::Texture>,
    pub cube_uv_view: Option<wgpu::TextureView>,
}

impl GpuCubeTexture {
    pub fn upload(device: &wgpu::Device, queue: &wgpu::Queue, src: &CubeTexture) -> Self {
        if let Some(atlas) = &src.cube_uv_atlas {
            return Self::upload_cube_uv(device, queue, src, atlas);
        }
        if let (Some(mips), Some(sizes)) = (&src.pmrem_mips, &src.pmrem_sizes) {
            return Self::upload_pmrem(device, queue, src.format, mips, sizes);
        }
        Self::upload_faces(device, queue, src.format, src.size, 1, &src.faces, 0)
    }

    fn upload_cube_uv(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        src: &CubeTexture,
        atlas: &CubeUvAtlas,
    ) -> Self {
        let wgpu_fmt = wgpu_format(src.format);
        let cube = Self::upload_faces(device, queue, src.format, src.size.max(1), 1, &src.faces, 0);
        let (uv_fmt, upload_bytes, bytes_per_row) = if let Some(f32_px) = &atlas.pixels_f32 {
            let (bytes, bpr) = pad_rows_rgba16f(f32_px, atlas.width, atlas.height);
            (wgpu::TextureFormat::Rgba16Float, bytes, bpr)
        } else {
            let bpp = 4u32;
            let (bytes, bpr) =
                pad_rows_for_upload(atlas.pixels.as_ref(), atlas.width, atlas.height, bpp);
            (wgpu_fmt, bytes, bpr)
        };
        let uv_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("threers cube uv atlas"),
            size: wgpu::Extent3d {
                width: atlas.width,
                height: atlas.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: uv_fmt,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &uv_tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &upload_bytes,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(atlas.height),
            },
            wgpu::Extent3d {
                width: atlas.width,
                height: atlas.height,
                depth_or_array_layers: 1,
            },
        );
        let uv_view = uv_tex.create_view(&wgpu::TextureViewDescriptor::default());
        Self {
            texture: cube.texture,
            view: cube.view,
            cube_uv_texture: Some(uv_tex),
            cube_uv_view: Some(uv_view),
        }
    }

    fn upload_pmrem(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: TextureFormat,
        mips: &[[Arc<Vec<u8>>; 6]],
        sizes: &[u32],
    ) -> Self {
        let base_size = sizes[0].max(1);
        let mip_count = sizes.len() as u32;
        Self::upload_mip_faces(device, queue, format, base_size, mip_count, mips, sizes)
    }

    fn upload_faces(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: TextureFormat,
        size: u32,
        mip_count: u32,
        faces: &[Arc<Vec<u8>>; 6],
        _base_level: u32,
    ) -> Self {
        let wgpu_fmt = wgpu_format(format);
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("threers cube texture"),
            size: wgpu::Extent3d {
                width: size.max(1),
                height: size.max(1),
                depth_or_array_layers: 6,
            },
            mip_level_count: mip_count,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu_fmt,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let bytes_per_pixel = match format {
            TextureFormat::Rgba8UnormSrgb | TextureFormat::Rgba8Unorm => 4,
            TextureFormat::R8Unorm => 1,
        };
        let bytes_per_row = size * bytes_per_pixel as u32;
        for (layer, face) in faces.iter().enumerate() {
            queue.write_texture(
                wgpu::ImageCopyTexture {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: 0,
                        y: 0,
                        z: layer as u32,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                face,
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(size),
                },
                wgpu::Extent3d {
                    width: size,
                    height: size,
                    depth_or_array_layers: 1,
                },
            );
        }
        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("threers cube view"),
            dimension: Some(wgpu::TextureViewDimension::Cube),
            ..Default::default()
        });
        Self {
            texture,
            view,
            cube_uv_texture: None,
            cube_uv_view: None,
        }
    }

    fn upload_mip_faces(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: TextureFormat,
        base_size: u32,
        mip_count: u32,
        mips: &[[Arc<Vec<u8>>; 6]],
        sizes: &[u32],
    ) -> Self {
        let wgpu_fmt = wgpu_format(format);
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("threers pmrem cube texture"),
            size: wgpu::Extent3d {
                width: base_size,
                height: base_size,
                depth_or_array_layers: 6,
            },
            mip_level_count: mip_count,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu_fmt,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let bytes_per_pixel = match format {
            TextureFormat::Rgba8UnormSrgb | TextureFormat::Rgba8Unorm => 4,
            TextureFormat::R8Unorm => 1,
        };
        for (level, (faces, size)) in mips.iter().zip(sizes.iter()).enumerate() {
            let size = *size;
            let bytes_per_row = size * bytes_per_pixel as u32;
            for (layer, face) in faces.iter().enumerate() {
                queue.write_texture(
                    wgpu::ImageCopyTexture {
                        texture: &texture,
                        mip_level: level as u32,
                        origin: wgpu::Origin3d {
                            x: 0,
                            y: 0,
                            z: layer as u32,
                        },
                        aspect: wgpu::TextureAspect::All,
                    },
                    face,
                    wgpu::ImageDataLayout {
                        offset: 0,
                        bytes_per_row: Some(bytes_per_row),
                        rows_per_image: Some(size),
                    },
                    wgpu::Extent3d {
                        width: size,
                        height: size,
                        depth_or_array_layers: 1,
                    },
                );
            }
        }
        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("threers pmrem cube view"),
            dimension: Some(wgpu::TextureViewDimension::Cube),
            ..Default::default()
        });
        Self {
            texture,
            view,
            cube_uv_texture: None,
            cube_uv_view: None,
        }
    }
}

pub fn cube_cache_key(t: &Arc<CubeTexture>) -> *const CubeTexture {
    Arc::as_ptr(t)
}

impl GpuTexture {
    pub fn upload(device: &wgpu::Device, queue: &wgpu::Queue, src: &Texture) -> Self {
        let format = wgpu_format(src.format);
        let size = wgpu::Extent3d {
            width: src.width.max(1),
            height: src.height.max(1),
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("threers texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let bytes_per_row = src.width * src.bytes_per_pixel() as u32;
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &src.data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(src.height),
            },
            size,
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Self { texture, view }
    }
}

pub fn wgpu_format(f: TextureFormat) -> wgpu::TextureFormat {
    match f {
        TextureFormat::Rgba8UnormSrgb => wgpu::TextureFormat::Rgba8UnormSrgb,
        TextureFormat::Rgba8Unorm => wgpu::TextureFormat::Rgba8Unorm,
        TextureFormat::R8Unorm => wgpu::TextureFormat::R8Unorm,
    }
}

pub fn wgpu_filter(f: TextureFilter) -> wgpu::FilterMode {
    match f {
        TextureFilter::Nearest => wgpu::FilterMode::Nearest,
        TextureFilter::Linear => wgpu::FilterMode::Linear,
    }
}

pub fn wgpu_wrap(w: TextureWrap) -> wgpu::AddressMode {
    match w {
        TextureWrap::ClampToEdge => wgpu::AddressMode::ClampToEdge,
        TextureWrap::Repeat => wgpu::AddressMode::Repeat,
        TextureWrap::MirroredRepeat => wgpu::AddressMode::MirrorRepeat,
    }
}

pub fn tex_cache_key(t: &Arc<Texture>) -> *const Texture {
    Arc::as_ptr(t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn f32_to_f16_matches_ieee_known_values() {
        for (value, expected) in [
            (0.0, 0x0000),
            (1.0, 0x3c00),
            (-1.0, 0xbc00),
            (2.0, 0x4000),
            (20.0, 0x4d00),
            (-9.0, 0xc880),
            (65504.0, 0x7bff),
        ] {
            assert_eq!(
                f32_to_f16_bits(value),
                expected,
                "incorrect f16 encoding for {value}"
            );
        }
    }

    #[test]
    fn f16_round_trip_preserves_probe_range() {
        for value in [-20.0, -9.0, -2.0, -0.1, 0.1, 2.0, 9.0, 20.0] {
            let round_trip = f16_bits_to_f32(f32_to_f16_bits(value));
            let relative_error = (round_trip - value).abs() / value.abs();
            assert!(
                relative_error < 0.001,
                "{value} became {round_trip} ({relative_error} relative error)"
            );
        }
    }
}
