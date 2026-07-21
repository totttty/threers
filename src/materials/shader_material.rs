//! User-defined custom-shader material — threers' extensibility escape hatch.
//!
//! A [`ShaderMaterial`] lets you plug arbitrary WGSL fragment code (with your own
//! uniform + storage data) into the renderer without editing the built-in shader
//! or the `Material` enum. It is compiled against threers' standard preamble, so
//! the fragment has the full built-in environment available:
//!
//! - the vertex output `VsOut` (`world_pos`, `world_normal`, `uv`, `vertex_color`,
//!   `view_z`, `clip_pos`, …),
//! - the `frame` uniform (camera, lights, ambient, fog, tone-mapping),
//! - the `mesh` uniform (model/normal matrices, color, params…),
//! - every built-in helper (`pbr_brdf`, `apply_fog`, `framebuffer_encode`, …),
//! - and a **user data group** at `@group(4)`:
//!
//! ```wgsl
//! struct ThreersUserData { data: array<vec4<f32>, 16>, };
//! @group(4) @binding(0) var<uniform> u_data: ThreersUserData;
//! @group(4) @binding(1) var<storage, read> u_s0: array<f32>;
//! @group(4) @binding(2) var<storage, read> u_s1: array<f32>;
//! @group(4) @binding(3) var<storage, read> u_s2: array<f32>;
//! ```
//!
//! You supply the fragment entry point:
//! ```wgsl
//! @fragment
//! fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
//!     let v = u_data.data[0].x;               // a scalar uniform
//!     let x = u_s0[0];                          // a storage element
//!     return vec4<f32>(in.vertex_color.rgb, 1.0);
//! }
//! ```

/// A custom-shader material. See the [module docs](self) for the WGSL contract.
#[derive(Debug, Clone)]
pub struct ShaderMaterial {
    /// WGSL defining `@fragment fn fs_main(in: VsOut) -> @location(0) vec4<f32>`.
    /// The standard preamble (structs, `vs_main`, helpers, `@group(4)` bindings)
    /// is prepended automatically.
    pub fragment: String,
    /// Up to 16 `vec4<f32>` uniform slots, exposed as `u_data.data[i]`.
    pub data: Vec<[f32; 4]>,
    /// Read-only `array<f32>` storage buffers, exposed as `u_s0`/`u_s1`/`u_s2`.
    pub storage0: Vec<f32>,
    pub storage1: Vec<f32>,
    pub storage2: Vec<f32>,
    pub opacity: f32,
    pub transparent: bool,
    /// `0` = FrontSide, `1` = BackSide, `2` = DoubleSide.
    pub side: u32,
}

impl Default for ShaderMaterial {
    fn default() -> Self {
        Self {
            fragment: String::new(),
            data: Vec::new(),
            storage0: Vec::new(),
            storage1: Vec::new(),
            storage2: Vec::new(),
            opacity: 1.0,
            transparent: false,
            side: 0,
        }
    }
}

impl ShaderMaterial {
    /// Create a material whose fragment stage is the given WGSL source
    /// (must define `@fragment fn fs_main`).
    pub fn new(fragment: impl Into<String>) -> Self {
        Self {
            fragment: fragment.into(),
            ..Default::default()
        }
    }

    /// Set the `@group(4)` uniform `vec4` slots (`u_data.data`).
    pub fn with_data(mut self, data: Vec<[f32; 4]>) -> Self {
        self.data = data;
        self
    }
    /// Set storage buffer `u_s0`.
    pub fn with_storage0(mut self, s: Vec<f32>) -> Self {
        self.storage0 = s;
        self
    }
    /// Set storage buffer `u_s1`.
    pub fn with_storage1(mut self, s: Vec<f32>) -> Self {
        self.storage1 = s;
        self
    }
    /// Set storage buffer `u_s2`.
    pub fn with_storage2(mut self, s: Vec<f32>) -> Self {
        self.storage2 = s;
        self
    }
    /// Material opacity (`0.0..=1.0`).
    pub fn with_opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity;
        self
    }
    /// Whether the material participates in alpha blending.
    pub fn with_transparent(mut self, transparent: bool) -> Self {
        self.transparent = transparent;
        self
    }
    /// Face culling: `0` front, `1` back, `2` double-sided.
    pub fn with_side(mut self, side: u32) -> Self {
        self.side = side;
        self
    }

    /// The 16 uniform slots as a flat `[[f32; 4]; 16]`, zero-padded/truncated.
    pub fn data_slots(&self) -> [[f32; 4]; 16] {
        let mut out = [[0.0f32; 4]; 16];
        for (i, v) in self.data.iter().take(16).enumerate() {
            out[i] = *v;
        }
        out
    }
}
