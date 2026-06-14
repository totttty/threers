use std::sync::Arc;
use crate::math::Vector2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureFormat {
    /// Standard 8-bit sRGB color (e.g. baseColor / albedo).
    Rgba8UnormSrgb,
    /// Linear 8-bit (e.g. roughness, metalness, normal maps).
    Rgba8Unorm,
    /// Single-channel 8-bit.
    R8Unorm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureFilter {
    Nearest,
    Linear,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureWrap {
    ClampToEdge,
    Repeat,
    MirroredRepeat,
}

/// 2D texture source. Mirrors three.js's `Texture` for the parts we need.
#[derive(Debug, Clone)]
pub struct Texture {
    pub width: u32,
    pub height: u32,
    pub format: TextureFormat,
    pub mag_filter: TextureFilter,
    pub min_filter: TextureFilter,
    pub wrap_s: TextureWrap,
    pub wrap_t: TextureWrap,
    /// Pixel data. Length must equal `width * height * bytes_per_pixel`.
    pub data: Arc<Vec<u8>>,
    pub flip_y: bool,
    /// UV transform — translation, rotation, scale all collapsed into a 2D
    /// affine. Mirrors three.js's `Texture.matrix`.
    pub offset: Vector2,
    pub repeat: Vector2,
    pub rotation: f32,
    /// If set, this texture is backed by a RenderTarget's color attachment
    /// instead of the CPU `data` bytes. The renderer skips upload and uses
    /// the RenderTarget's view directly for sampling.
    pub external_rt_id: Option<u32>,
}

impl Texture {
    pub fn new(width: u32, height: u32, format: TextureFormat, data: Vec<u8>) -> Self {
        Self {
            width,
            height,
            format,
            mag_filter: TextureFilter::Linear,
            min_filter: TextureFilter::Linear,
            wrap_s: TextureWrap::ClampToEdge,
            wrap_t: TextureWrap::ClampToEdge,
            data: Arc::new(data),
            flip_y: true,
            offset: Vector2::ZERO,
            repeat: Vector2::ONE,
            rotation: 0.0,
            external_rt_id: None,
        }
    }

    /// Build a Texture that's backed by a RenderTarget's color attachment.
    /// The renderer skips upload and samples directly from the RT view.
    pub fn from_render_target(rt_id: u32, width: u32, height: u32, format: TextureFormat) -> Self {
        Self {
            width,
            height,
            format,
            mag_filter: TextureFilter::Linear,
            min_filter: TextureFilter::Linear,
            wrap_s: TextureWrap::ClampToEdge,
            wrap_t: TextureWrap::ClampToEdge,
            data: Arc::new(Vec::new()),
            flip_y: false,
            offset: Vector2::ZERO,
            repeat: Vector2::ONE,
            rotation: 0.0,
            external_rt_id: Some(rt_id),
        }
    }

    pub fn bytes_per_pixel(&self) -> usize {
        match self.format {
            TextureFormat::Rgba8UnormSrgb | TextureFormat::Rgba8Unorm => 4,
            TextureFormat::R8Unorm => 1,
        }
    }

    /// Build a solid-color 1x1 texture in linear RGBA8. Useful as a placeholder
    /// when a material slot is unused.
    pub fn solid(rgba: [u8; 4], format: TextureFormat) -> Self {
        Self::new(1, 1, format, rgba.to_vec())
    }
}
