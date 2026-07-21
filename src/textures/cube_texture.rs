use super::{TextureFilter, TextureFormat, TextureWrap};
use std::sync::Arc;

/// CPU-side CubeUV PMREM atlas (three.js `CubeUVReflectionMapping` layout).
#[derive(Debug, Clone)]
pub struct CubeUvAtlas {
    pub width: u32,
    pub height: u32,
    pub cube_size: u32,
    pub lod_max: u32,
    pub texel_width: f32,
    pub texel_height: f32,
    pub pixels: Arc<Vec<u8>>,
    /// Linear f32 RGBA atlas before 8-bit quantize (PMREM blur output). GPU uploads this.
    pub pixels_f32: Option<Arc<Vec<f32>>>,
}

/// Cubemap with six faces in the order +X, -X, +Y, -Y, +Z, -Z.
/// Each face must have the same width and height.
#[derive(Debug, Clone)]
pub struct CubeTexture {
    pub size: u32,
    pub format: TextureFormat,
    pub mag_filter: TextureFilter,
    pub min_filter: TextureFilter,
    pub wrap: TextureWrap,
    /// 6 faces. Each face's length must equal `size * size * bytes_per_pixel`.
    pub faces: [Arc<Vec<u8>>; 6],
    /// Optional PMREM mip chain: each entry is six faces at decreasing resolution.
    /// When present, GPU upload writes a full mip pyramid for roughness-based IBL.
    pub pmrem_mips: Option<Vec<[Arc<Vec<u8>>; 6]>>,
    pub pmrem_sizes: Option<Vec<u32>>,
    /// CubeUV PMREM atlas (three.js `CubeUVReflectionMapping` layout).
    pub cube_uv_atlas: Option<CubeUvAtlas>,
}

impl CubeTexture {
    pub fn new(size: u32, format: TextureFormat, faces: [Vec<u8>; 6]) -> Self {
        let [f0, f1, f2, f3, f4, f5] = faces;
        Self {
            size,
            format,
            mag_filter: TextureFilter::Linear,
            min_filter: TextureFilter::Linear,
            wrap: TextureWrap::ClampToEdge,
            faces: [
                Arc::new(f0),
                Arc::new(f1),
                Arc::new(f2),
                Arc::new(f3),
                Arc::new(f4),
                Arc::new(f5),
            ],
            pmrem_mips: None,
            pmrem_sizes: None,
            cube_uv_atlas: None,
        }
    }

    pub fn with_pmrem_mips(mut self, mips: Vec<[Vec<u8>; 6]>, sizes: Vec<u32>) -> Self {
        self.pmrem_mips = Some(
            mips.into_iter()
                .map(|f| {
                    let [a, b, c, d, e, g] = f;
                    [
                        Arc::new(a),
                        Arc::new(b),
                        Arc::new(c),
                        Arc::new(d),
                        Arc::new(e),
                        Arc::new(g),
                    ]
                })
                .collect(),
        );
        self.pmrem_sizes = Some(sizes);
        self
    }

    pub fn with_cube_uv_atlas(mut self, atlas: CubeUvAtlas) -> Self {
        self.cube_uv_atlas = Some(atlas);
        self
    }

    pub fn is_cube_uv(&self) -> bool {
        self.cube_uv_atlas.is_some()
    }

    pub fn mip_level_count(&self) -> u32 {
        self.pmrem_sizes
            .as_ref()
            .map(|s| s.len() as u32)
            .unwrap_or(1)
    }
}
