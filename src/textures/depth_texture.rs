use super::{TextureFilter, TextureWrap};

/// Sampled depth texture (e.g. for shadow map / depth-based effects).
/// The renderer creates the underlying wgpu texture itself; this struct just
/// records the requested size and sampler params.
#[derive(Debug, Clone, Copy)]
pub struct DepthTexture {
    pub width: u32,
    pub height: u32,
    pub mag_filter: TextureFilter,
    pub min_filter: TextureFilter,
    pub wrap: TextureWrap,
}

impl DepthTexture {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            mag_filter: TextureFilter::Nearest,
            min_filter: TextureFilter::Nearest,
            wrap: TextureWrap::ClampToEdge,
        }
    }
}
