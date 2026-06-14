//! wgpu scene renderer: pipelines, resource caches, shadows, and post-processing.
//!
//! Walks a [`Scene`](crate::Scene), batches draw calls by material and topology,
//! and writes to a swapchain view or [`super::RenderTarget`]. Post-fx passes
//! share WGSL in [`super::shader`] (`POSTFX_SHADER`, effect kinds 0–30).

use std::collections::HashMap;
use std::sync::Arc;
use bytemuck::{Pod, Zeroable};

use crate::core::{BufferGeometry, ObjectKind};
use crate::cameras::Camera;
use crate::lights::Light;
use crate::materials::{Material, MaterialKind, MaterialTextureSlots};
use crate::math::{Matrix3, Matrix4, Vector3};
use crate::scene::Scene;
use crate::textures::{Texture, TextureFormat, CubeTexture};
use super::gpu_mesh::{GpuMesh, geom_cache_key};
use super::gpu_texture::{GpuTexture, GpuCubeTexture, tex_cache_key, cube_cache_key};
use super::shader::{
    SHADER_SOURCE,
    MAX_DIR_LIGHTS, MAX_POINT_LIGHTS, MAX_SPOT_LIGHTS, MAX_HEMI_LIGHTS,
};

fn rt_sample_format(format: wgpu::TextureFormat) -> wgpu::TextureFormat {
    if format == wgpu::TextureFormat::Rgba16Float {
        format
    } else {
        format.add_srgb_suffix()
    }
}

// ---------- GPU layout types ----------

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, Default)]
struct DirLightGpu {
    direction: [f32; 4],
    color: [f32; 4],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, Default)]
struct PointLightGpu {
    position: [f32; 4],
    color: [f32; 4],
    params: [f32; 4],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, Default)]
struct SpotLightGpu {
    position: [f32; 4],
    direction: [f32; 4],
    color: [f32; 4],
    params: [f32; 4],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, Default)]
struct HemiLightGpu {
    sky_color: [f32; 4],
    ground_color: [f32; 4],
    direction: [f32; 4],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct FrameUniforms {
    view: [f32; 16],
    projection: [f32; 16],
    view_proj: [f32; 16],
    shadow_vp: [f32; 16],
    spot_shadow_vp: [f32; 16],
    cube_face_vp: [f32; 16],
    camera_position: [f32; 4],
    ambient: [f32; 4],
    light_counts: [u32; 4],
    /// x: dir shadow on, y: dir bias, z: spot shadow on, w: spot bias
    shadow_params: [f32; 4],
    /// xyz: point shadow caster world position, w: radius (0 = disabled)
    point_shadow_pos: [f32; 4],
    /// x: tone-mapping mode (0 none, 1 linear, 2 ACES), y: exposure, z: IBL on,
    /// w: linear framebuffer (1 = HalfFloat RT, skip sRGB encode on write)
    tone_mapping_exposure: [f32; 4],
    /// Fog color rgb + a (a unused). Multiplied with the mix factor in shader.
    fog_color: [f32; 4],
    /// x: near (linear), y: far (linear), z: density (exp2), w: mode (0/1/2)
    fog_params: [f32; 4],
    /// x: viewport width (pixels), y: viewport height (pixels), zw: reserved.
    viewport_size: [f32; 4],
    /// CubeUV PMREM: x/y texel size, z max mip, w = 1 when CubeUV active.
    env_map_params: [f32; 4],
    dir_lights: [DirLightGpu; MAX_DIR_LIGHTS],
    point_lights: [PointLightGpu; MAX_POINT_LIGHTS],
    spot_lights: [SpotLightGpu; MAX_SPOT_LIGHTS],
    hemi_lights: [HemiLightGpu; MAX_HEMI_LIGHTS],
}

impl Default for FrameUniforms {
    fn default() -> Self {
        Self {
            view: [0.0; 16],
            projection: [0.0; 16],
            view_proj: [0.0; 16],
            shadow_vp: [0.0; 16],
            spot_shadow_vp: [0.0; 16],
            cube_face_vp: [0.0; 16],
            camera_position: [0.0; 4],
            ambient: [0.0; 4],
            light_counts: [0; 4],
            shadow_params: [0.0; 4],
            point_shadow_pos: [0.0; 4],
            // Default: NoToneMapping, exposure 1.0 — matches three.js default.
            tone_mapping_exposure: [0.0, 1.0, 0.0, 0.0],
            fog_color: [1.0, 1.0, 1.0, 1.0],
            fog_params: [1.0, 1000.0, 0.0, 0.0],
            viewport_size: [1.0, 1.0, 0.0, 0.0],
            env_map_params: [0.0; 4],
            dir_lights: [DirLightGpu::default(); MAX_DIR_LIGHTS],
            point_lights: [PointLightGpu::default(); MAX_POINT_LIGHTS],
            spot_lights: [SpotLightGpu::default(); MAX_SPOT_LIGHTS],
            hemi_lights: [HemiLightGpu::default(); MAX_HEMI_LIGHTS],
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, Default)]
struct MeshUniforms {
    model: [f32; 16],
    normal_matrix: [f32; 16],
    color: [f32; 4],
    emissive: [f32; 4],
    specular: [f32; 4],
    /// x: shininess/depth_near, y: opacity/depth_far, z: roughness, w: metalness
    params: [f32; 4],
    /// x: ao_intensity, y/z: normal_scale, w: toon_steps (bitcast u32 in WGSL)
    params2: [f32; 4],
    /// Physical layers: x: clearcoat, y: clearcoat_roughness, z: ior, w: transmission
    params3: [f32; 4],
    /// x: material kind, y: texture flags
    flags: [u32; 4],
}

// Texture-flag bits — keep in sync with shader.rs FLAG_*.
const FLAG_MAP: u32           = 1;
const FLAG_NORMAL_MAP: u32    = 2;
const FLAG_ROUGHNESS_MAP: u32 = 4;
const FLAG_METALNESS_MAP: u32 = 8;
const FLAG_AO_MAP: u32        = 16;
const FLAG_EMISSIVE_MAP: u32  = 32;
const FLAG_MATCAP_MAP: u32    = 64;
const FLAG_DASHED: u32        = 128;
const FLAG_SHADOW_MAT: u32    = 256;
const FLAG_RECEIVE_SHADOW: u32 = 512;

struct CachedGpuMesh {
    version: u32,
    mesh: GpuMesh,
}

/// Camera parameters forwarded into SSAO / SSR post-fx uniforms.
#[derive(Clone)]
pub struct PostFxCamera {
    pub near: f32,
    pub far: f32,
    pub kernel_radius: f32,
    pub kernel_size: u32,
    pub proj: [f32; 16],
    pub inv_proj: [f32; 16],
}

impl Default for PostFxCamera {
    fn default() -> Self {
        let mut id = [0f32; 16];
        id[0] = 1.0; id[5] = 1.0; id[10] = 1.0; id[15] = 1.0;
        Self { near: 0.1, far: 100.0, kernel_radius: 8.0, kernel_size: 32, proj: id, inv_proj: id }
    }
}


/// GPU renderer: scene draws, shadow maps, environment sampling, post-fx.
pub struct Renderer {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    pipeline_tri: wgpu::RenderPipeline,
    pipeline_tri_alpha: wgpu::RenderPipeline,
    /// No-cull triangle pipeline, used for `side: BackSide` / `side: DoubleSide`
    /// materials. Without this, three.js apps using BackSide for skybox-style
    /// "inside of a box" rendering silently get culled-to-nothing.
    pipeline_tri_nocull: wgpu::RenderPipeline,
    /// Sky dome: no cull, no depth write, LessEqual (matches three.js Sky depthWrite:false).
    pipeline_tri_sky: wgpu::RenderPipeline,
    pipeline_tri_wire: wgpu::RenderPipeline,
    pipeline_line: wgpu::RenderPipeline,
    pipeline_point: wgpu::RenderPipeline,
    pipeline_sprite: wgpu::RenderPipeline,
    /// Half-float color-target variants for EffectComposer / postfx RTs.
    pipeline_tri_f16: wgpu::RenderPipeline,
    pipeline_tri_alpha_f16: wgpu::RenderPipeline,
    pipeline_tri_nocull_f16: wgpu::RenderPipeline,
    pipeline_tri_sky_f16: wgpu::RenderPipeline,
    pipeline_tri_wire_f16: wgpu::RenderPipeline,
    pipeline_line_f16: wgpu::RenderPipeline,
    pipeline_point_f16: wgpu::RenderPipeline,
    pipeline_sprite_f16: wgpu::RenderPipeline,
    pipeline_instanced_f16: wgpu::RenderPipeline,
    pipeline_skinned_f16: wgpu::RenderPipeline,
    pipeline_shadow: wgpu::RenderPipeline,
    pipeline_shadow_spot: wgpu::RenderPipeline,
    pipeline_shadow_point: wgpu::RenderPipeline,
    pipeline_instanced: wgpu::RenderPipeline,
    pipeline_skinned: wgpu::RenderPipeline,
    /// Post-fx fullscreen-quad pipeline. Reads one input texture, writes to
    /// one output attachment, applies the effect selected by the postfx
    /// uniform's effect-kind field.
    pipeline_postfx: wgpu::RenderPipeline,
    /// Additive-blend variant of the postfx pipeline. Used for bloom composite
    /// where the bloom contribution adds to the existing framebuffer content.
    pipeline_postfx_add: wgpu::RenderPipeline,
    /// Outline overlay: SrcAlpha + One (three.js AdditiveBlending).
    pipeline_postfx_outline_add: wgpu::RenderPipeline,
    /// Multiply-blend variant (DstColor * SrcColor). Used for SSAO composite.
    pipeline_postfx_multiply: wgpu::RenderPipeline,
    /// Half-float postfx variants for outline intermediates (Rgba16Float RTs).
    pipeline_postfx_f16: wgpu::RenderPipeline,
    pipeline_postfx_add_f16: wgpu::RenderPipeline,
    postfx_bgl: wgpu::BindGroupLayout,
    postfx_sampler: wgpu::Sampler,
    postfx_nearest_sampler: wgpu::Sampler,
    /// Repeat + nearest sampler for SSAO noise (matches three.js RepeatWrapping).
    postfx_noise_sampler: wgpu::Sampler,
    /// Uploaded by SSAOPass — 32 hemisphere samples (vec4 stride).
    ssao_kernel_buffer: wgpu::Buffer,
    ssao_noise_tex: wgpu::Texture,
    ssao_noise_view: wgpu::TextureView,
    /// 64×64 R8 displacement map for GlitchPass (DigitalGlitch tDisp).
    glitch_disp_tex: wgpu::Texture,
    glitch_disp_view: wgpu::TextureView,
    /// Full-screen R8 snow map for GlitchPass (precomputed in JS for GLSL parity).
    glitch_snow_tex: wgpu::Texture,
    glitch_snow_view: wgpu::TextureView,
    /// CPU-uploaded RGBA8 for WebGL postfx fallback blit to canvas/RT.
    rgba_upload_tex: wgpu::Texture,
    rgba_upload_view: wgpu::TextureView,
    mesh_skinned_bgl: wgpu::BindGroupLayout,
    sprite_quad: GpuMesh,
    frame_bgl: wgpu::BindGroupLayout,
    mesh_bgl: wgpu::BindGroupLayout,
    tex_bgl: wgpu::BindGroupLayout,
    env_bgl: wgpu::BindGroupLayout,
    depth_view: wgpu::TextureView,
    /// 1×1 placeholder depth texture for postfx binding 6 when hw depth is unused.
    hw_depth_tex: wgpu::Texture,
    depth_size: (u32, u32),

    geom_cache: HashMap<*const BufferGeometry, CachedGpuMesh>,
    skin_attr_cache: HashMap<*const BufferGeometry, wgpu::Buffer>,
    tex_cache: HashMap<*const Texture, GpuTexture>,
    /// Lookup table for render-target-backed textures, keyed by RT id.
    rt_view_cache: HashMap<u32, wgpu::TextureView>,
    /// Linear (non-sRGB-decode) sample views for AO/depth/normal postfx buffers.
    rt_linear_view_cache: HashMap<u32, wgpu::TextureView>,
    /// Render targets keyed by id (for creating per-pass sample views).
    rt_registry: HashMap<u32, Arc<super::RenderTarget>>,
    /// GPU depth-buffer views for SSAO (Depth32Float, depth-only aspect).
    rt_depth_view_cache: HashMap<u32, wgpu::TextureView>,
    /// Lookup table for cube render targets' cube views, keyed by RT id.
    cube_rt_view_cache: HashMap<u32, wgpu::TextureView>,
    cube_cache: HashMap<*const CubeTexture, GpuCubeTexture>,
    env_bind_group: wgpu::BindGroup,
    shadow_env_bind_group: wgpu::BindGroup,
    env_sampler: wgpu::Sampler,
    default_env_cube: GpuCubeTexture,
    default_cube_uv_view: wgpu::TextureView,
    env_cached_key: Option<*const CubeTexture>,
    /// When the env source is a CubeRenderTarget, this is its id. Used to
    /// invalidate the env bind group when the RT changes.
    env_cached_rt_id: Option<u32>,
    shadow_view: wgpu::TextureView,
    shadow_sampler: wgpu::Sampler,
    spot_shadow_view: wgpu::TextureView,
    spot_shadow_sampler: wgpu::Sampler,
    #[allow(dead_code)]
    point_shadow_cube_view: wgpu::TextureView,
    point_shadow_face_views: [wgpu::TextureView; 6],
    #[allow(dead_code)]
    point_shadow_sampler: wgpu::Sampler,

    frame_buffer: wgpu::Buffer,
    frame_bind_group: wgpu::BindGroup,
    per_mesh: Vec<(wgpu::Buffer, wgpu::BindGroup, wgpu::BindGroup)>, // uniform, mesh bg, tex bg

    sampler_linear: wgpu::Sampler,
    sampler_nearest_clamp: wgpu::Sampler,
    sampler_linear_repeat: wgpu::Sampler,
    sampler_nearest_repeat: wgpu::Sampler,

    // Default fallbacks (1x1 textures used when a material slot is empty).
    default_white_srgb: GpuTexture,
    default_white_linear: GpuTexture,
    default_normal: GpuTexture, // (0.5, 0.5, 1.0, 1.0) — straight up tangent normal
    #[allow(dead_code)]
    default_black: GpuTexture,

    color_format: wgpu::TextureFormat,
}

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

impl Renderer {
    /// Shared device handle. Used by callers (e.g. async readback) that need
    /// to encode commands or allocate buffers outside the renderer's own loop.
    pub fn device_arc(&self) -> Arc<wgpu::Device> { self.device.clone() }
    /// Shared queue handle (same use case as `device_arc`).
    pub fn queue_arc(&self) -> Arc<wgpu::Queue> { self.queue.clone() }

    fn ensure_geometry(&mut self, geom: &std::sync::Arc<BufferGeometry>) {
        let key = geom_cache_key(geom);
        let ver = geom.geometry_version;
        let stale = self.geom_cache.get(&key).map(|e| e.version != ver).unwrap_or(true);
        if stale {
            let mesh = GpuMesh::upload(&self.device, geom);
            self.geom_cache.insert(key, CachedGpuMesh { version: ver, mesh });
        }
    }

    /// Create pipelines and default 1×1 fallback textures.
    ///
    /// `color_format` is the swapchain or render-target format (typically
    /// `Rgba8Unorm` or `Rgba8UnormSrgb` on web, `Bgra8UnormSrgb` on some desktops).
    pub fn new(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        color_format: wgpu::TextureFormat,
        width: u32,
        height: u32,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("threers shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER_SOURCE.into()),
        });

        let uniform_entry = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };

        let frame_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("threers frame bgl"),
            entries: &[uniform_entry(0)],
        });

        let mesh_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("threers mesh bgl"),
            entries: &[uniform_entry(0)],
        });

        // Skinned variant of the mesh BGL: same uniform at binding 0, plus a
        // storage buffer of bone matrices at binding 1. This lets the skinned
        // pipeline stay within the 4-bind-group WebGPU default limit.
        let mesh_skinned_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("threers mesh skinned bgl"),
            entries: &[
                uniform_entry(0),
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        // 7 textures (albedo, normal, roughness, metalness, ao, emissive, matcap) + 1 sampler
        let tex_entry = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let tex_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("threers tex bgl"),
            entries: &[
                tex_entry(0), tex_entry(1), tex_entry(2),
                tex_entry(3), tex_entry(4), tex_entry(5), tex_entry(6),
                wgpu::BindGroupLayoutEntry {
                    binding: 7,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let env_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("threers env bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::Cube,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 6,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::Cube,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 7,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 8,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("threers layout"),
            bind_group_layouts: &[&frame_bgl, &mesh_bgl, &tex_bgl, &env_bgl],
            push_constant_ranges: &[],
        });

        // Skinned pipeline layout: same as the base 4 groups, but group 1 uses
        // the skinned mesh BGL (uniform + bones storage). Stays at 4 groups —
        // WebGPU's default `maxBindGroups` limit.
        let pipeline_layout_skinned = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("threers skinned layout"),
            bind_group_layouts: &[&frame_bgl, &mesh_skinned_bgl, &tex_bgl, &env_bgl],
            push_constant_ranges: &[],
        });

        let vertex_buffers = [wgpu::VertexBufferLayout {
            array_stride: GpuMesh::VERTEX_STRIDE as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute { offset:  0, shader_location: 0, format: wgpu::VertexFormat::Float32x3 },
                wgpu::VertexAttribute { offset: 12, shader_location: 1, format: wgpu::VertexFormat::Float32x3 },
                wgpu::VertexAttribute { offset: 24, shader_location: 2, format: wgpu::VertexFormat::Float32x2 },
                wgpu::VertexAttribute { offset: 32, shader_location: 3, format: wgpu::VertexFormat::Float32x4 },
            ],
        }];

        let make_pipeline = |fmt: wgpu::TextureFormat, topology: wgpu::PrimitiveTopology, cull: Option<wgpu::Face>, label: &str| {
            let alpha = label.contains("alpha");
            // PolygonMode::Line needs the POLYGON_MODE_LINE feature, which is
            // generally unavailable in browser WebGPU. The wireframe pipeline
            // instead uses LineList primitives over the triangle indices —
            // we'll feed it edge indices at draw time.
            let polygon_mode = wgpu::PolygonMode::Fill;
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: "vs_main",
                    buffers: &vertex_buffers,
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: "fs_main",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: fmt,
                        blend: Some(if alpha { wgpu::BlendState::ALPHA_BLENDING } else { wgpu::BlendState::REPLACE }),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: cull,
                    polygon_mode,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH_FORMAT,
                    // Transparent: don't write depth, so back fragments behind a
                    // transparent surface still composite correctly.
                    depth_write_enabled: !alpha,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
            })
        };
        let f16_format = wgpu::TextureFormat::Rgba16Float;
        let pipeline_tri       = make_pipeline(color_format, wgpu::PrimitiveTopology::TriangleList, Some(wgpu::Face::Back), "threers tri pipeline");
        let pipeline_tri_alpha = make_pipeline(color_format, wgpu::PrimitiveTopology::TriangleList, Some(wgpu::Face::Back), "threers tri alpha pipeline");
        let pipeline_tri_nocull = make_pipeline(color_format, wgpu::PrimitiveTopology::TriangleList, None,                   "threers tri nocull pipeline");
        let make_sky_pipeline = |fmt: wgpu::TextureFormat, label: &str| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: "vs_main",
                    buffers: &vertex_buffers,
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: "fs_main",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: fmt,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH_FORMAT,
                    depth_write_enabled: false,
                    depth_compare: wgpu::CompareFunction::LessEqual,
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
            })
        };
        let pipeline_tri_sky = make_sky_pipeline(color_format, "threers tri sky pipeline");
        // PolygonMode::Line draws the triangle outline — used for `wireframe:true` materials.
        // wgpu requires the NON_FILL_POLYGON_MODE feature for Line/Point polygon modes on
        // some backends. If unsupported, this falls back to a no-op (renders nothing extra),
        // but Chromium's WebGPU/SwiftShader path tends to allow it.
        let pipeline_tri_wire  = make_pipeline(color_format, wgpu::PrimitiveTopology::LineList,     None,                   "threers tri wire pipeline");
        let pipeline_line      = make_pipeline(color_format, wgpu::PrimitiveTopology::LineList,     None,                   "threers line pipeline");
        let pipeline_point     = make_pipeline(color_format, wgpu::PrimitiveTopology::PointList,    None,                   "threers point pipeline");
        let pipeline_tri_f16       = make_pipeline(f16_format, wgpu::PrimitiveTopology::TriangleList, Some(wgpu::Face::Back), "threers tri f16 pipeline");
        let pipeline_tri_alpha_f16 = make_pipeline(f16_format, wgpu::PrimitiveTopology::TriangleList, Some(wgpu::Face::Back), "threers tri alpha f16 pipeline");
        let pipeline_tri_nocull_f16 = make_pipeline(f16_format, wgpu::PrimitiveTopology::TriangleList, None, "threers tri nocull f16 pipeline");
        let pipeline_tri_sky_f16 = make_sky_pipeline(f16_format, "threers tri sky f16 pipeline");
        let pipeline_tri_wire_f16  = make_pipeline(f16_format, wgpu::PrimitiveTopology::LineList, None, "threers tri wire f16 pipeline");
        let pipeline_line_f16      = make_pipeline(f16_format, wgpu::PrimitiveTopology::LineList, None, "threers line f16 pipeline");
        let pipeline_point_f16     = make_pipeline(f16_format, wgpu::PrimitiveTopology::PointList, None, "threers point f16 pipeline");

        // Instanced pipeline: vertex layout slot 0 (per-vertex), slot 1 per-instance mat4.
        let vertex_buffers_instanced = [
            wgpu::VertexBufferLayout {
                array_stride: GpuMesh::VERTEX_STRIDE as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[
                    wgpu::VertexAttribute { offset:  0, shader_location: 0, format: wgpu::VertexFormat::Float32x3 },
                    wgpu::VertexAttribute { offset: 12, shader_location: 1, format: wgpu::VertexFormat::Float32x3 },
                    wgpu::VertexAttribute { offset: 24, shader_location: 2, format: wgpu::VertexFormat::Float32x2 },
                    wgpu::VertexAttribute { offset: 32, shader_location: 3, format: wgpu::VertexFormat::Float32x4 },
                ],
            },
            wgpu::VertexBufferLayout {
                array_stride: 64,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &[
                    wgpu::VertexAttribute { offset:  0, shader_location: 4, format: wgpu::VertexFormat::Float32x4 },
                    wgpu::VertexAttribute { offset: 16, shader_location: 5, format: wgpu::VertexFormat::Float32x4 },
                    wgpu::VertexAttribute { offset: 32, shader_location: 6, format: wgpu::VertexFormat::Float32x4 },
                    wgpu::VertexAttribute { offset: 48, shader_location: 7, format: wgpu::VertexFormat::Float32x4 },
                ],
            },
        ];
        // Skinned vertex layout: slot 0 = standard vertex, slot 1 = joints+weights.
        let vertex_buffers_skinned = [
            wgpu::VertexBufferLayout {
                array_stride: GpuMesh::VERTEX_STRIDE as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[
                    wgpu::VertexAttribute { offset:  0, shader_location: 0, format: wgpu::VertexFormat::Float32x3 },
                    wgpu::VertexAttribute { offset: 12, shader_location: 1, format: wgpu::VertexFormat::Float32x3 },
                    wgpu::VertexAttribute { offset: 24, shader_location: 2, format: wgpu::VertexFormat::Float32x2 },
                    wgpu::VertexAttribute { offset: 32, shader_location: 3, format: wgpu::VertexFormat::Float32x4 },
                ],
            },
            wgpu::VertexBufferLayout {
                array_stride: 32,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[
                    wgpu::VertexAttribute { offset:  0, shader_location: 4, format: wgpu::VertexFormat::Float32x4 },
                    wgpu::VertexAttribute { offset: 16, shader_location: 5, format: wgpu::VertexFormat::Float32x4 },
                ],
            },
        ];
        let pipeline_skinned = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("threers skinned pipeline"),
            layout: Some(&pipeline_layout_skinned),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_skinned",
                buffers: &vertex_buffers_skinned,
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: color_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });
        let pipeline_skinned_f16 = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("threers skinned f16 pipeline"),
            layout: Some(&pipeline_layout_skinned),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_skinned",
                buffers: &vertex_buffers_skinned,
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: f16_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        let pipeline_instanced = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("threers instanced pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_instanced",
                buffers: &vertex_buffers_instanced,
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: color_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });
        let pipeline_instanced_f16 = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("threers instanced f16 pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_instanced",
                buffers: &vertex_buffers_instanced,
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: f16_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        let pipeline_sprite = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("threers sprite pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_sprite",
                buffers: &vertex_buffers,
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: color_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });
        let pipeline_sprite_f16 = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("threers sprite f16 pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_sprite",
                buffers: &vertex_buffers,
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: f16_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        // Depth-only pipeline used to render the scene from each shadow-casting
        // light's POV. Reuses the per-vertex layout + per-mesh uniforms.
        let make_shadow_pipeline = |entry: &str, label: &str| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: entry,
                    buffers: &vertex_buffers,
                    compilation_options: Default::default(),
                },
                fragment: None,
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(wgpu::Face::Front),
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH_FORMAT,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::LessEqual,
                    stencil: Default::default(),
                    bias: wgpu::DepthBiasState {
                        constant: 2,
                        slope_scale: 2.0,
                        clamp: 0.0,
                    },
                }),
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
            })
        };
        let pipeline_shadow = make_shadow_pipeline("vs_shadow", "threers shadow pipeline");
        let pipeline_shadow_spot = make_shadow_pipeline("vs_shadow_spot", "threers spot shadow pipeline");
        let pipeline_shadow_point = make_shadow_pipeline("vs_shadow_point", "threers point shadow pipeline");

        // Post-fx fullscreen-quad pipeline. Reads one input texture, writes
        // to a render attachment matching the surface format.
        let postfx_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("threers postfx shader"),
            source: wgpu::ShaderSource::Wgsl(crate::renderer::shader::POSTFX_SHADER.into()),
        });
        let postfx_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("threers postfx bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 6,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 7,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 8,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 9,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                    count: None,
                },
            ],
        });
        let postfx_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("threers postfx pipeline layout"),
            bind_group_layouts: &[&postfx_bgl],
            push_constant_ranges: &[],
        });
        let pipeline_postfx = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("threers postfx pipeline"),
            layout: Some(&postfx_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &postfx_shader,
                entry_point: "vs_main",
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &postfx_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: color_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });
        let postfx_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("threers postfx sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let postfx_nearest_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("threers postfx nearest sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let postfx_noise_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("threers postfx noise sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let ssao_kernel_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("threers ssao kernel buffer"),
            size: (32 * 16) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let ssao_noise_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("threers ssao noise"),
            size: wgpu::Extent3d { width: 4, height: 4, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let ssao_noise_view = ssao_noise_tex.create_view(&wgpu::TextureViewDescriptor::default());
        let glitch_disp_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("threers glitch disp"),
            size: wgpu::Extent3d { width: 64, height: 64, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let glitch_disp_view = glitch_disp_tex.create_view(&wgpu::TextureViewDescriptor::default());
        let glitch_snow_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("threers glitch snow"),
            size: wgpu::Extent3d { width: 800, height: 600, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let glitch_snow_view = glitch_snow_tex.create_view(&wgpu::TextureViewDescriptor::default());
        let rgba_upload_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("threers rgba upload"),
            size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let rgba_upload_view = rgba_upload_tex.create_view(&wgpu::TextureViewDescriptor::default());
        // Same pipeline as `pipeline_postfx`, but with additive blending so
        // the shader output adds to existing framebuffer pixels (bloom composite).
        let pipeline_postfx_add = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("threers postfx additive pipeline"),
            layout: Some(&postfx_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &postfx_shader,
                entry_point: "vs_main",
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &postfx_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: color_format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent::REPLACE,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });
        let pipeline_postfx_outline_add = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("threers postfx outline additive pipeline"),
            layout: Some(&postfx_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &postfx_shader,
                entry_point: "vs_main",
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &postfx_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: color_format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent::REPLACE,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });
        let pipeline_postfx_multiply = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("threers postfx multiply pipeline"),
            layout: Some(&postfx_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &postfx_shader,
                entry_point: "vs_main",
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &postfx_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: color_format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::Dst,
                            dst_factor: wgpu::BlendFactor::Zero,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent::REPLACE,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });
        let postfx_f16_format = wgpu::TextureFormat::Rgba16Float;
        let pipeline_postfx_f16 = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("threers postfx f16 pipeline"),
            layout: Some(&postfx_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &postfx_shader,
                entry_point: "vs_main",
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &postfx_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: postfx_f16_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });
        let pipeline_postfx_add_f16 = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("threers postfx additive f16 pipeline"),
            layout: Some(&postfx_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &postfx_shader,
                entry_point: "vs_main",
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &postfx_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: postfx_f16_format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent::REPLACE,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        let sprite_quad = {
            let mut g = crate::core::BufferGeometry::new();
            g.set_attribute("position", crate::core::BufferAttribute::new(vec![
                -0.5, -0.5, 0.0,
                 0.5, -0.5, 0.0,
                 0.5,  0.5, 0.0,
                -0.5,  0.5, 0.0,
            ], 3));
            g.set_attribute("uv", crate::core::BufferAttribute::new(vec![
                0.0, 1.0,  1.0, 1.0,  1.0, 0.0,  0.0, 0.0,
            ], 2));
            g.set_index(vec![0, 1, 2, 0, 2, 3]);
            GpuMesh::upload(&device, &g)
        };

        let depth_view = create_depth_view(&device, width, height);
        let hw_depth_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("threers postfx hw depth placeholder"),
            size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let frame_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("threers frame uniform"),
            size: std::mem::size_of::<FrameUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let frame_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("threers frame bg"),
            layout: &frame_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: frame_buffer.as_entire_binding(),
            }],
        });

        let sampler_linear = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("threers linear sampler"),
            // three.js's DataTexture default is ClampToEdge — matching here
            // prevents UV-1.0-corner samples from wrapping back to UV 0.
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let sampler_nearest_clamp = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("threers nearest-clamp sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let sampler_linear_repeat = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("threers linear-repeat sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let sampler_nearest_repeat = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("threers nearest-repeat sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let default_white_srgb = GpuTexture::upload(&device, &queue,
            &Texture::solid([255, 255, 255, 255], TextureFormat::Rgba8UnormSrgb));
        let default_white_linear = GpuTexture::upload(&device, &queue,
            &Texture::solid([255, 255, 255, 255], TextureFormat::Rgba8Unorm));
        let default_normal = GpuTexture::upload(&device, &queue,
            &Texture::solid([128, 128, 255, 255], TextureFormat::Rgba8Unorm));
        let default_black = GpuTexture::upload(&device, &queue,
            &Texture::solid([0, 0, 0, 255], TextureFormat::Rgba8Unorm));

        // Default 1x1 environment cubemap: all-black faces (zero env contribution).
        let default_env_cube = {
            let face_data = [0u8; 4];
            let faces = [
                face_data.to_vec(), face_data.to_vec(), face_data.to_vec(),
                face_data.to_vec(), face_data.to_vec(), face_data.to_vec(),
            ];
            let src = CubeTexture::new(1, TextureFormat::Rgba8UnormSrgb, faces);
            GpuCubeTexture::upload(&device, &queue, &src)
        };
        let env_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("threers env sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        // Shadow depth texture for the directional light's depth pre-pass.
        // 1024×1024 directional shadow map.
        const SHADOW_SIZE: u32 = 1024;
        let shadow_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("threers shadow depth"),
            size: wgpu::Extent3d { width: SHADOW_SIZE, height: SHADOW_SIZE, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let shadow_view = shadow_tex.create_view(&wgpu::TextureViewDescriptor::default());
        let shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("threers shadow sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            compare: Some(wgpu::CompareFunction::LessEqual),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let spot_shadow_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("threers spot shadow depth"),
            size: wgpu::Extent3d { width: SHADOW_SIZE, height: SHADOW_SIZE, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let spot_shadow_view = spot_shadow_tex.create_view(&wgpu::TextureViewDescriptor::default());
        let spot_shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("threers spot shadow sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            compare: Some(wgpu::CompareFunction::LessEqual),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // Point shadow cubemap depth (6 layers × 512×512 Depth32Float).
        const POINT_SHADOW_SIZE: u32 = 512;
        let point_shadow_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("threers point shadow depth cube"),
            size: wgpu::Extent3d { width: POINT_SHADOW_SIZE, height: POINT_SHADOW_SIZE, depth_or_array_layers: 6 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let point_shadow_cube_view = point_shadow_tex.create_view(&wgpu::TextureViewDescriptor {
            label: Some("threers point shadow cube view"),
            dimension: Some(wgpu::TextureViewDimension::Cube),
            ..Default::default()
        });
        let mk_face = |i: u32| point_shadow_tex.create_view(&wgpu::TextureViewDescriptor {
            label: Some("threers point shadow face"),
            dimension: Some(wgpu::TextureViewDimension::D2),
            base_array_layer: i,
            array_layer_count: Some(1),
            ..Default::default()
        });
        let point_shadow_face_views: [wgpu::TextureView; 6] = [
            mk_face(0), mk_face(1), mk_face(2),
            mk_face(3), mk_face(4), mk_face(5),
        ];
        let point_shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("threers point shadow sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            compare: Some(wgpu::CompareFunction::LessEqual),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let default_cube_uv_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("threers default cube uv"),
            size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::ImageCopyTexture { texture: &default_cube_uv_tex, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            &[0u8, 0, 0, 255],
            wgpu::ImageDataLayout { offset: 0, bytes_per_row: Some(4), rows_per_image: Some(1) },
            wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
        );
        let default_cube_uv_view = default_cube_uv_tex.create_view(&wgpu::TextureViewDescriptor::default());

        let env_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("threers env bg"),
            layout: &env_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&default_env_cube.view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&env_sampler) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&shadow_view) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::Sampler(&shadow_sampler) },
                wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::TextureView(&spot_shadow_view) },
                wgpu::BindGroupEntry { binding: 5, resource: wgpu::BindingResource::Sampler(&spot_shadow_sampler) },
                wgpu::BindGroupEntry { binding: 6, resource: wgpu::BindingResource::TextureView(&point_shadow_cube_view) },
                wgpu::BindGroupEntry { binding: 7, resource: wgpu::BindingResource::Sampler(&point_shadow_sampler) },
                wgpu::BindGroupEntry { binding: 8, resource: wgpu::BindingResource::TextureView(&default_cube_uv_view) },
            ],
        });

        // Dummy 1x1 depth textures used by `shadow_env_bind_group` so the
        // shadow passes can satisfy the pipeline layout without binding the
        // textures they're currently writing to (WebGPU forbids simultaneous
        // RenderAttachment + TextureBinding usage of the same resource).
        let dummy_depth_2d = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("threers dummy depth 2d"),
            size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
            mip_level_count: 1, sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let dummy_depth_2d_view = dummy_depth_2d.create_view(&wgpu::TextureViewDescriptor::default());
        let dummy_depth_cube = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("threers dummy depth cube"),
            size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 6 },
            mip_level_count: 1, sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let dummy_depth_cube_view = dummy_depth_cube.create_view(&wgpu::TextureViewDescriptor {
            label: Some("threers dummy depth cube view"),
            dimension: Some(wgpu::TextureViewDimension::Cube),
            ..Default::default()
        });
        let shadow_env_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("threers shadow env bg (dummies)"),
            layout: &env_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&default_env_cube.view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&env_sampler) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&dummy_depth_2d_view) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::Sampler(&shadow_sampler) },
                wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::TextureView(&dummy_depth_2d_view) },
                wgpu::BindGroupEntry { binding: 5, resource: wgpu::BindingResource::Sampler(&spot_shadow_sampler) },
                wgpu::BindGroupEntry { binding: 6, resource: wgpu::BindingResource::TextureView(&dummy_depth_cube_view) },
                wgpu::BindGroupEntry { binding: 7, resource: wgpu::BindingResource::Sampler(&point_shadow_sampler) },
                wgpu::BindGroupEntry { binding: 8, resource: wgpu::BindingResource::TextureView(&default_cube_uv_view) },
            ],
        });

        Self {
            device,
            queue,
            pipeline_tri,
            pipeline_tri_alpha,
            pipeline_tri_nocull,
            pipeline_tri_sky,
            pipeline_tri_wire,
            pipeline_line,
            pipeline_point,
            pipeline_sprite,
            pipeline_tri_f16,
            pipeline_tri_alpha_f16,
            pipeline_tri_nocull_f16,
            pipeline_tri_sky_f16,
            pipeline_tri_wire_f16,
            pipeline_line_f16,
            pipeline_point_f16,
            pipeline_sprite_f16,
            pipeline_instanced_f16,
            pipeline_skinned_f16,
            pipeline_shadow,
            pipeline_shadow_spot,
            pipeline_shadow_point,
            pipeline_instanced,
            pipeline_skinned,
            pipeline_postfx,
            pipeline_postfx_add,
            pipeline_postfx_outline_add,
            pipeline_postfx_multiply,
            pipeline_postfx_f16,
            pipeline_postfx_add_f16,
            postfx_bgl,
            postfx_sampler,
            postfx_nearest_sampler,
            postfx_noise_sampler,
            ssao_kernel_buffer,
            ssao_noise_tex,
            ssao_noise_view,
            glitch_disp_tex,
            glitch_disp_view,
            glitch_snow_tex,
            glitch_snow_view,
            rgba_upload_tex,
            rgba_upload_view,
            mesh_skinned_bgl,
            sprite_quad,
            frame_bgl,
            mesh_bgl,
            tex_bgl,
            env_bgl,
            depth_view,
            hw_depth_tex,
            depth_size: (width, height),
            geom_cache: HashMap::new(),
            skin_attr_cache: HashMap::new(),
            tex_cache: HashMap::new(),
            rt_view_cache: HashMap::new(),
            rt_linear_view_cache: HashMap::new(),
            rt_registry: HashMap::new(),
            rt_depth_view_cache: HashMap::new(),
            cube_rt_view_cache: HashMap::new(),
            cube_cache: HashMap::new(),
            env_bind_group,
            shadow_env_bind_group,
            env_sampler,
            default_env_cube,
            default_cube_uv_view,
            env_cached_key: None,
            env_cached_rt_id: None,
            shadow_view,
            shadow_sampler,
            spot_shadow_view,
            spot_shadow_sampler,
            point_shadow_cube_view,
            point_shadow_face_views,
            point_shadow_sampler,
            frame_buffer,
            frame_bind_group,
            per_mesh: Vec::new(),
            sampler_linear,
            sampler_nearest_clamp,
            sampler_linear_repeat,
            sampler_nearest_repeat,
            default_white_srgb,
            default_white_linear,
            default_normal,
            default_black,
            color_format,
        }
    }

    pub fn color_format(&self) -> wgpu::TextureFormat { self.color_format }

    pub fn resize(&mut self, width: u32, height: u32) {
        if (width, height) != self.depth_size && width > 0 && height > 0 {
            self.depth_view = create_depth_view(&self.device, width, height);
            self.depth_size = (width, height);
        }
    }

    /// Apply a post-fx pass from a registered render-target id (wasm path).
    ///
    /// Same as [`Self::apply_postfx`] but resolves `input_rt_id` internally so
    /// the caller does not hold a borrow against `&mut self`. When `additive`
    /// is true, output is blended onto existing framebuffer pixels.
    ///
    /// `kind` selects the WGSL branch in [`super::shader::POSTFX_SHADER`].
    /// `time` carries effect-specific data (e.g. glitch RNG seed). `params2` /
    /// `params3` are four-vectors whose meaning depends on `kind`.
    pub fn apply_postfx_by_id(
        &mut self,
        input_rt_id: u32,
        output_rt_id: u32,
        output_view: &wgpu::TextureView,
        depth_rt_id: u32,
        normal_rt_id: u32,
        kind: u32,
        time: f32,
        params2: [f32; 4],
        params3: [f32; 4],
        width: u32,
        height: u32,
        additive: bool,
        camera: PostFxCamera,
    ) {
        let output_format = if output_rt_id != 0 {
            self.rt_registry.get(&output_rt_id).map(|rt| rt.format).unwrap_or(self.color_format)
        } else {
            self.color_format
        };
        let input_view = if let Some(v) = self.rt_view_cache.remove(&input_rt_id) {
            v
        } else if let Some(rt) = self.rt_registry.get(&input_rt_id) {
            rt.color_texture.create_view(&wgpu::TextureViewDescriptor {
                label: Some("threers postfx input fallback"),
                format: Some(rt_sample_format(rt.format)),
                ..Default::default()
            })
        } else {
            return;
        };
        let depth_owned = if depth_rt_id != 0 && depth_rt_id != input_rt_id {
            self.rt_view_cache.remove(&depth_rt_id).or_else(|| {
                self.rt_registry.get(&depth_rt_id).map(|rt| {
                    rt.color_texture.create_view(&wgpu::TextureViewDescriptor {
                        label: Some("threers postfx depth fallback"),
                        format: Some(rt_sample_format(rt.format)),
                        ..Default::default()
                    })
                })
            })
        } else {
            None
        };
        let normal_owned = if normal_rt_id != 0 {
            self.rt_view_cache.remove(&normal_rt_id)
        } else {
            None
        };
        let depth_ref = depth_owned.as_ref().unwrap_or(&input_view);
        let input_is_f16 = self.rt_registry.get(&input_rt_id)
            .map(|rt| rt.format == wgpu::TextureFormat::Rgba16Float)
            .unwrap_or(false);
        // Kind 0 copies stored RT texels 1:1. Default: linear (non-sRGB-decode)
        // view so textureLoad returns the gamma-encoded bytes the main shader
        // wrote (matches direct-to-canvas). params2.y > 0.5 selects the legacy
        // sRGB-decode view for OutlinePass scene copy (three.js darkens there).
        let kind0_legacy = kind == 0 && params2[1] > 0.5;
        let output_is_f16 = output_format == wgpu::TextureFormat::Rgba16Float;
        let kind0_byte_copy = kind == 0 && !kind0_legacy && !input_is_f16 && !output_is_f16;
        let use_linear_input = if kind == 0 {
            input_is_f16 && !kind0_legacy
        } else {
            (matches!(kind, 13 | 15) && !kind0_legacy)
                || input_is_f16
                || matches!(kind, 16 | 18 | 19)
        };
        let use_linear_depth = matches!(kind, 11 | 12 | 14);
        let input_linear = if use_linear_input {
            self.rt_registry.get(&input_rt_id).map(|rt| {
                rt.color_texture.create_view(&wgpu::TextureViewDescriptor {
                    label: Some("threers postfx linear input"),
                    format: Some(rt.format),
                    ..Default::default()
                })
            })
        } else { None };
        let input_raw = if kind0_byte_copy {
            self.rt_registry.get(&input_rt_id).map(|rt| {
                rt.color_texture.create_view(&wgpu::TextureViewDescriptor {
                    label: Some("threers postfx raw input"),
                    format: Some(rt.format),
                    ..Default::default()
                })
            })
        } else { None };
        let depth_linear = if use_linear_depth && depth_rt_id != 0 {
            self.rt_registry.get(&depth_rt_id).map(|rt| {
                rt.color_texture.create_view(&wgpu::TextureViewDescriptor {
                    label: Some("threers postfx linear depth"),
                    format: Some(rt.format),
                    ..Default::default()
                })
            })
        } else { None };
        let normal_linear = if use_linear_depth {
            self.rt_registry.get(&normal_rt_id).map(|rt| {
                rt.color_texture.create_view(&wgpu::TextureViewDescriptor {
                    label: Some("threers postfx linear normal"),
                    format: Some(rt.format),
                    ..Default::default()
                })
            })
        } else { None };
        let input_bind = input_raw.as_ref().or(input_linear.as_ref()).unwrap_or(&input_view);
        let depth_bind = depth_linear.as_ref().unwrap_or(depth_ref);
        let mut params2 = params2;
        if kind0_byte_copy {
            params2[2] = 1.0;
        }
        let use_hw_depth = kind == 11 && depth_rt_id != 0 && depth_rt_id == normal_rt_id;
        let hw_depth_local = if use_hw_depth {
            self.rt_depth_view_cache.remove(&normal_rt_id)
        } else {
            None
        };
        let hw_depth_fallback = self.hw_depth_tex.create_view(&wgpu::TextureViewDescriptor {
            label: Some("threers postfx hw depth fallback"),
            aspect: wgpu::TextureAspect::DepthOnly,
            ..Default::default()
        });
        let hw_depth_bind = hw_depth_local.as_ref().unwrap_or(&hw_depth_fallback);
        let flip_y = if output_rt_id == 0 {
            if kind == 0 {
                !kind0_byte_copy && params2[1] > 0.5
            } else if matches!(kind, 4 | 5 | 25 | 26) {
                // Halftone/glitch: pixel-space sampling on same-orientation RT.
                false
            } else {
                true
            }
        } else {
            false
        };
        match normal_owned {
            Some(n) => {
                let normal_bind = normal_linear.as_ref().unwrap_or(&n);
                self.apply_postfx_ex(
                    input_bind, output_view, depth_bind, normal_bind, hw_depth_bind,
                    kind, time, params2, params3, width, height, additive, camera, use_hw_depth,
                    output_format,
                    flip_y,
                );
                self.rt_view_cache.insert(normal_rt_id, n);
            }
            None => {
                let fallback_normal = self.default_normal.texture.create_view(
                    &wgpu::TextureViewDescriptor::default(),
                );
                let normal_bind = normal_linear.as_ref().unwrap_or(&fallback_normal);
                self.apply_postfx_ex(
                    input_bind, output_view, depth_bind, normal_bind, hw_depth_bind,
                    kind, time, params2, params3, width, height, additive, camera, use_hw_depth,
                    output_format,
                    flip_y,
                );
            }
        }
        if let Some(v) = hw_depth_local {
            self.rt_depth_view_cache.insert(normal_rt_id, v);
        }
        self.rt_view_cache.insert(input_rt_id, input_view);
        if let Some(d) = depth_owned {
            self.rt_view_cache.insert(depth_rt_id, d);
        }
    }

    /// Apply a post-fx pass: read from `input_view`, write to `output_view`.
    /// Defaults to REPLACE blend / clear on load. Use `apply_postfx_ex` for
    /// additive blend (bloom composite) or load-preserve behavior.
    pub fn apply_postfx(
        &mut self,
        input_view: &wgpu::TextureView,
        output_view: &wgpu::TextureView,
        kind: u32,
        time: f32,
        params2: [f32; 4],
        params3: [f32; 4],
        width: u32,
        height: u32,
    ) {
        let fallback_normal = self.default_normal.texture.create_view(
            &wgpu::TextureViewDescriptor::default(),
        );
        let hw_fallback = self.hw_depth_tex.create_view(&wgpu::TextureViewDescriptor {
            aspect: wgpu::TextureAspect::DepthOnly,
            ..Default::default()
        });
        self.apply_postfx_ex(
            input_view, output_view, input_view, &fallback_normal, &hw_fallback,
            kind, time, params2, params3, width, height, false, PostFxCamera::default(), false,
            self.color_format,
            !matches!(kind, 4 | 5 | 25 | 26),
        );
    }

    /// Extended post-fx pass with `additive` blend toggle.
    pub fn apply_postfx_ex(
        &mut self,
        input_view: &wgpu::TextureView,
        output_view: &wgpu::TextureView,
        depth_view: &wgpu::TextureView,
        normal_view: &wgpu::TextureView,
        hw_depth_view: &wgpu::TextureView,
        kind: u32,
        time: f32,
        params2: [f32; 4],
        params3: [f32; 4],
        width: u32,
        height: u32,
        additive: bool,
        camera: PostFxCamera,
        use_hw_depth: bool,
        output_format: wgpu::TextureFormat,
        flip_y: bool,
    ) {
        let linear_output = output_format == wgpu::TextureFormat::Rgba16Float;
        #[repr(C)]
        #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
        struct PostFxUniforms {
            params: [f32; 4],
            params2: [f32; 4],
            params3: [f32; 4],
            resolution: [f32; 4],
            ssao0: [f32; 4],
            inv_proj: [f32; 16],
            proj: [f32; 16],
        }
        let uniforms = PostFxUniforms {
            params: [
                kind as f32,
                time,
                if kind == 5 || kind == 25 || kind == 26 { params3[3] } else { 0.0 },
                if use_hw_depth { 1.0 } else { 0.0 },
            ],
            params2,
            params3,
            resolution: [
                width as f32,
                height as f32,
                if linear_output { 1.0 } else { 0.0 },
                if flip_y { 1.0 } else { 0.0 },
            ],
            ssao0: [camera.near, camera.far, camera.kernel_radius, camera.kernel_size as f32],
            inv_proj: camera.inv_proj,
            proj: camera.proj,
        };
        let buffer = wgpu::util::DeviceExt::create_buffer_init(
            &*self.device,
            &wgpu::util::BufferInitDescriptor {
                label: Some("threers postfx uniforms"),
                contents: bytemuck::cast_slice(&[uniforms]),
                usage: wgpu::BufferUsages::UNIFORM,
            },
        );
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("threers postfx bg"),
            layout: &self.postfx_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(input_view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&self.postfx_sampler) },
                wgpu::BindGroupEntry { binding: 2, resource: buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(depth_view) },
                wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::Sampler(
                    if kind == 5 || kind == 25 || kind == 26 { &self.postfx_sampler } else { &self.postfx_nearest_sampler }
                ) },
                wgpu::BindGroupEntry { binding: 5, resource: wgpu::BindingResource::TextureView(normal_view) },
                wgpu::BindGroupEntry { binding: 6, resource: wgpu::BindingResource::TextureView(hw_depth_view) },
                wgpu::BindGroupEntry { binding: 7, resource: self.ssao_kernel_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 8, resource: wgpu::BindingResource::TextureView(
                    if kind == 26 {
                        &self.glitch_snow_view
                    } else if kind == 5 || kind == 25 {
                        &self.glitch_disp_view
                    } else if kind == 3 && params3[3] > 0.5 {
                        &self.glitch_snow_view
                    } else {
                        &self.ssao_noise_view
                    }
                ) },
                wgpu::BindGroupEntry { binding: 9, resource: wgpu::BindingResource::Sampler(
                    &self.postfx_noise_sampler
                ) },
            ],
        });
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("threers postfx encoder"),
        });
        {
            let multiply = kind == 15;
            let load_op = if additive || multiply {
                wgpu::LoadOp::Load
            } else {
                wgpu::LoadOp::Clear(wgpu::Color::BLACK)
            };
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("threers postfx pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: output_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: load_op,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            let output_is_f16 = output_format == wgpu::TextureFormat::Rgba16Float;
            let pipe = if multiply {
                &self.pipeline_postfx_multiply
            } else if additive {
                if kind == 17 {
                    &self.pipeline_postfx_outline_add
                } else if output_is_f16 {
                    &self.pipeline_postfx_add_f16
                } else {
                    &self.pipeline_postfx_add
                }
            } else if output_is_f16 {
                &self.pipeline_postfx_f16
            } else {
                &self.pipeline_postfx
            };
            pass.set_pipeline(pipe);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
        self.queue.submit(std::iter::once(encoder.finish()));
    }

    /// Upload SSAO hemisphere kernel samples (32 vec3 values, padded to vec4).
    pub fn set_ssao_kernel(&mut self, kernel: &[f32]) {
        let mut data = [0f32; 128];
        for i in 0..32 {
            let base = i * 3;
            if base + 2 < kernel.len() {
                data[i * 4] = kernel[base];
                data[i * 4 + 1] = kernel[base + 1];
                data[i * 4 + 2] = kernel[base + 2];
            }
        }
        self.queue.write_buffer(&self.ssao_kernel_buffer, 0, bytemuck::cast_slice(&data));
    }

    /// Upload GLSL-baked dotscreen pattern (R32Float; binding 8 when kind=3 params3.w>0.5).
    pub fn set_dotscreen_pattern(&mut self, data: &[f32], width: u32, height: u32) {
        self.set_glitch_snow(data, width, height);
    }

    /// Blit tightly-packed RGBA8 bytes (row-major, top-first) to an output view.
    pub fn blit_rgba8(
        &mut self,
        data: &[u8],
        width: u32,
        height: u32,
        output_view: &wgpu::TextureView,
        output_format: wgpu::TextureFormat,
    ) {
        let w = width.max(1);
        let h = height.max(1);
        let need = (w * h * 4) as usize;
        if data.len() < need {
            return;
        }
        if self.rgba_upload_tex.width() != w || self.rgba_upload_tex.height() != h {
            self.rgba_upload_tex = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("threers rgba upload"),
                size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            self.rgba_upload_view = self.rgba_upload_tex.create_view(
                &wgpu::TextureViewDescriptor::default(),
            );
        }
        const ALIGN: u32 = 256;
        let unpadded = w * 4;
        let padded = ((unpadded + ALIGN - 1) / ALIGN) * ALIGN;
        let mut upload = vec![0u8; (padded * h) as usize];
        for row in 0..h {
            let src = (row * w * 4) as usize;
            let dst = (row * padded) as usize;
            upload[dst..dst + (w * 4) as usize].copy_from_slice(&data[src..src + (w * 4) as usize]);
        }
        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.rgba_upload_tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &upload,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(padded),
                rows_per_image: Some(h),
            },
            wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        );
        let upload_view = self.rgba_upload_tex.create_view(
            &wgpu::TextureViewDescriptor::default(),
        );
        let fallback_normal = self.default_normal.texture.create_view(
            &wgpu::TextureViewDescriptor::default(),
        );
        let hw_fallback = self.hw_depth_tex.create_view(&wgpu::TextureViewDescriptor {
            aspect: wgpu::TextureAspect::DepthOnly,
            ..Default::default()
        });
        self.apply_postfx_ex(
            &upload_view,
            output_view,
            &upload_view,
            &fallback_normal,
            &hw_fallback,
            0,
            0.0,
            [1.0, 0.0, 0.0, 0.0],
            [0.0; 4],
            w,
            h,
            false,
            PostFxCamera::default(),
            false,
            output_format,
            true,
        );
    }

    /// Upload full-screen glitch snow map (R32Float, point-sampled).
    pub fn set_glitch_snow(&mut self, data: &[f32], width: u32, height: u32) {
        let w = width.max(1);
        let h = height.max(1);
        if self.glitch_snow_tex.width() != w || self.glitch_snow_tex.height() != h {
            self.glitch_snow_tex = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("threers glitch snow"),
                size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::R32Float,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            self.glitch_snow_view = self.glitch_snow_tex.create_view(&wgpu::TextureViewDescriptor::default());
        }
        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.glitch_snow_tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytemuck::cast_slice(data),
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(w * 4),
                rows_per_image: Some(h),
            },
            wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        );
    }

    /// Upload 64×64 glitch displacement heightmap (R32Float; manual bilinear in WGSL).
    pub fn set_glitch_disp(&mut self, data: &[f32], size: u32) {
        let sz = size.max(1);
        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.glitch_disp_tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytemuck::cast_slice(data),
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(sz * 4),
                rows_per_image: Some(sz),
            },
            wgpu::Extent3d { width: sz, height: sz, depth_or_array_layers: 1 },
        );
    }

    /// Upload 4×4 SSAO rotation noise (R channel, repeat-wrapped).
    pub fn set_ssao_noise(&mut self, noise: &[f32]) {
        let mut texels = [0f32; 16];
        for (dst, src) in texels.iter_mut().zip(noise.iter()) {
            *dst = *src;
        }
        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.ssao_noise_tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytemuck::cast_slice(&texels),
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(16),
                rows_per_image: Some(4),
            },
            wgpu::Extent3d { width: 4, height: 4, depth_or_array_layers: 1 },
        );
    }

    /// Render `scene` from `camera` into one face of a cube render target.
    /// Used by `CubeCamera.update()` which calls this 6 times per frame.
    pub fn render_to_cube_face(
        &mut self,
        scene: &mut crate::scene::Scene,
        camera: &dyn Camera,
        target: &super::CubeRenderTarget,
        face: usize,
    ) {
        let (orig_w, orig_h) = self.depth_size;
        let orig_depth = std::mem::replace(
            &mut self.depth_view,
            target.depth_texture.create_view(&wgpu::TextureViewDescriptor::default()),
        );
        self.depth_size = (target.side, target.side);
        let face_view = &target.face_views[face % 6];
        self.render(scene, camera, face_view, false);
        self.depth_view = orig_depth;
        self.depth_size = (orig_w, orig_h);
    }

    /// Render `scene` from `camera` into an offscreen [`RenderTarget`]. Uses
    /// the target's own depth buffer (required even when RT size matches the canvas).
    pub fn render_to(&mut self, scene: &mut crate::scene::Scene, camera: &dyn Camera, target: &super::RenderTarget) {
        let (orig_w, orig_h) = self.depth_size;
        let orig_depth = std::mem::replace(&mut self.depth_view, target.depth_view_clone());
        self.depth_size = (target.width, target.height);
        let linear_fb = target.format == wgpu::TextureFormat::Rgba16Float;
        self.render(scene, camera, &target.color_view, linear_fb);
        self.depth_view = orig_depth;
        self.depth_size = (orig_w, orig_h);
    }

    /// Draw `scene` from `camera` into `target_view`.
    /// `linear_framebuffer`: when true, skip sRGB encode (HalfFloat / postfx RT).
    pub fn render(&mut self, scene: &mut Scene, camera: &dyn Camera, target_view: &wgpu::TextureView, linear_framebuffer: bool) {
        scene.update_world();

        let view_m = camera.view_matrix();
        let proj_m = camera.projection_matrix();
        let view_proj = proj_m.multiply(&view_m).elements;

        // tone_mapping_exposure[2] is the "IBL on" flag — the PBR shader's
        // env-map branch is gated on it. Enable whenever the scene has any
        // environment source (CPU cube texture or cube render target).
        let has_env = scene.environment.is_some() || scene.environment_cube_rt.is_some();
        let (cam_near, cam_far) = camera.near_far();
        let mut frame_u = FrameUniforms {
            view: view_m.elements,
            projection: proj_m.elements,
            view_proj,
            camera_position: [camera.position().x, camera.position().y, camera.position().z, 0.0],
            fog_color: [scene.fog.color.r, scene.fog.color.g, scene.fog.color.b, 1.0],
            fog_params: [scene.fog.near, scene.fog.far, scene.fog.density, scene.fog.mode as f32],
            tone_mapping_exposure: [0.0, 1.0, if has_env { 1.0 } else { 0.0 }, if linear_framebuffer { 1.0 } else { 0.0 }],
            viewport_size: [self.depth_size.0 as f32, self.depth_size.1 as f32, cam_near, cam_far],
            ..FrameUniforms::default()
        };

        let mut ambient = [0.0_f32; 3];
        let mut n_dir = 0usize;
        let mut n_point = 0usize;
        let mut n_spot = 0usize;
        let mut n_hemi = 0usize;
        let mut shadow_caster: Option<(Vector3, crate::lights::ShadowSettings)> = None;
        let mut spot_caster: Option<(Vector3, Vector3, f32, crate::lights::ShadowSettings)> = None;
        let mut point_caster: Option<(Vector3, f32, crate::lights::ShadowSettings)> = None;

        #[derive(Clone, Copy, PartialEq, Eq)]
        enum Topology { Triangle, TriangleAlpha, TriangleWire, TriangleNoCull, Sky, Line, #[allow(dead_code)] Point, Sprite, Instanced, Skinned }
        struct DrawMesh {
            key: *const BufferGeometry,
            topology: Topology,
            /// Z in view space, used to back-to-front sort transparent draws.
            view_z: f32,
            render_order: i32,
            cast_shadow: bool,
            alpha_test: f32,
            model: [f32; 16],
            normal_matrix: [f32; 16],
            color: [f32; 4],
            emissive: [f32; 4],
            specular: [f32; 4],
            shininess_or_near: f32,
            opacity_or_far: f32,
            roughness: f32,
            metalness: f32,
            ao_intensity: f32,
            normal_scale: [f32; 2],
            toon_steps: u32,
            physical: [f32; 4],
            shader_flags: u32,
            kind: MaterialKind,
            slots: MaterialTextureSlots,
            /// Index into `instance_buffers` for Topology::Instanced.
            instance_buf_idx: usize,
            instance_count: u32,
            /// Index into `skin_data` for Topology::Skinned.
            skin_idx: usize,
        }
        let mut draws: Vec<DrawMesh> = Vec::new();
        let mut instance_buffers: Vec<wgpu::Buffer> = Vec::new();
        struct SkinData { bone_buf: wgpu::Buffer, bg: Option<wgpu::BindGroup> }
        let mut skin_data: Vec<SkinData> = Vec::new();

        // Helper: build a DrawMesh from a geometry+material pair under a given topology.
        let make_draw = |
            geom: &std::sync::Arc<BufferGeometry>,
            mat: &Material,
            obj: &crate::core::Object3D,
            topology: Topology,
        | -> DrawMesh {
            let key = geom_cache_key(geom);
            let nm3 = Matrix3::normal_matrix(&obj.matrix_world);
            // Mirror materials repurpose the normal_matrix slot for the
            // projective texture matrix supplied per-frame from JS.
            let normal_matrix = match mat {
                Material::Mirror(m) => m.texture_matrix,
                _ => mat3_to_mat4_array(&nm3),
            };
            // Promote Triangle to TriangleAlpha / TriangleWire based on material
            // flags. Pre-set view_z for later sort.
            let world_pos = obj.world_position();
            let view_z = view_m.elements[2] * world_pos.x
                + view_m.elements[6] * world_pos.y
                + view_m.elements[10] * world_pos.z
                + view_m.elements[14];
            let topology = if topology == Topology::Triangle {
                if matches!(mat, Material::Sky(_)) { Topology::Sky }
                else if mat.wireframe() { Topology::TriangleWire }
                else if mat.transparent() { Topology::TriangleAlpha }
                else if mat.side() != 0 { Topology::TriangleNoCull }
                else { Topology::Triangle }
            } else { topology };
            let c = mat.color();
            let e = mat.emissive();
            // For MeshDistanceMaterial, pack the reference position into the
            // specular channel (unused by that path) so the shader can read it.
            // For SkyMaterial, pack the sun position into specular.
            let s = match mat {
                Material::Distance(m) => crate::math::Color::new(
                    m.reference_position.x, m.reference_position.y, m.reference_position.z),
                Material::Sky(m) => crate::math::Color::new(
                    m.sun_position.x, m.sun_position.y, m.sun_position.z),
                _ => mat.specular(),
            };
            // For SkyMaterial, pack atmospheric tuning into params.x/y (replacing
            // shininess/opacity which it doesn't use).
            let (shin_or_near, opacity_or_far) = match mat {
                Material::Depth(m) => (m.near, m.far),
                Material::Distance(m) => (m.near_distance, m.far_distance),
                Material::Sky(m) => (m.turbidity, m.rayleigh),
                _ => (mat.shininess(), mat.opacity()),
            };
            let physical = match mat {
                Material::Physical(m) => [m.clearcoat, m.clearcoat_roughness, m.ior, m.transmission],
                Material::Line(m) => [m.dash_scale, m.dash_size, m.gap_size, 0.0],
                _ => [0.0; 4],
            };
            let mut shader_flags = match mat {
                Material::Line(m) if m.dash_size > 0.0 || m.gap_size > 0.0 => FLAG_DASHED,
                Material::Basic(m) if m.shadow_only => FLAG_SHADOW_MAT,
                _ => 0,
            };
            if obj.receive_shadow {
                shader_flags |= FLAG_RECEIVE_SHADOW;
            }
            DrawMesh {
                key,
                topology,
                view_z,
                render_order: obj.render_order,
                cast_shadow: obj.cast_shadow,
                alpha_test: mat.alpha_test(),
                model: obj.matrix_world.elements,
                normal_matrix,
                color: [c.r, c.g, c.b, mat.opacity()],
                emissive: [e.r, e.g, e.b, 0.0],
                specular: [s.r, s.g, s.b, 0.0],
                shininess_or_near: shin_or_near,
                opacity_or_far,
                // For SkyMaterial, repurpose roughness/metalness as
                // params.z/w (mieCoefficient/mieDirectionalG).
                roughness: match mat {
                    Material::Sky(m) => m.mie_coefficient,
                    _ => mat.roughness(),
                },
                metalness: match mat {
                    Material::Sky(m) => m.mie_directional_g,
                    _ => mat.metalness(),
                },
                ao_intensity: mat.ao_intensity(),
                normal_scale: [mat.normal_scale().x, mat.normal_scale().y],
                toon_steps: mat.toon_steps(),
                physical,
                shader_flags,
                kind: mat.kind(),
                slots: mat.texture_slots(),
                instance_buf_idx: usize::MAX,
                instance_count: 1,
                skin_idx: usize::MAX,
            }
        };

        let cam_layers = camera.layers();
        let root = scene.root;
        scene.arena.traverse_visible(root, &mut |_id, obj| {
            if !cam_layers.test(&obj.layers) { return; }
            match &obj.kind {
                ObjectKind::Light(light) => match light {
                    Light::Ambient(l) => {
                        ambient[0] += l.color.r * l.intensity;
                        ambient[1] += l.color.g * l.intensity;
                        ambient[2] += l.color.b * l.intensity;
                    }
                    Light::Directional(l) => {
                        if n_dir >= MAX_DIR_LIGHTS { return; }
                        // three.js: direction = normalize(target - position); default target (0,0,0).
                        let pos = obj.world_position();
                        let dir = {
                            let d = Vector3::new(-pos.x, -pos.y, -pos.z);
                            if d.length_sq() > 1e-8 {
                                d.normalize()
                            } else {
                                transform_direction(&obj.matrix_world, l.direction).normalize()
                            }
                        };
                        let c = l.color;
                        frame_u.dir_lights[n_dir] = DirLightGpu {
                            direction: [dir.x, dir.y, dir.z, 0.0],
                            color: [c.r * l.intensity, c.g * l.intensity, c.b * l.intensity, 0.0],
                        };
                        // First cast_shadow dir light becomes the shadow caster.
                        if l.cast_shadow && n_dir == 0 && shadow_caster.is_none() {
                            shadow_caster = Some((dir, l.shadow));
                        }
                        n_dir += 1;
                    }
                    Light::Point(l) => {
                        if n_point >= MAX_POINT_LIGHTS { return; }
                        let p = obj.world_position();
                        let c = l.color;
                        frame_u.point_lights[n_point] = PointLightGpu {
                            position: [p.x, p.y, p.z, 0.0],
                            color: [c.r * l.intensity, c.g * l.intensity, c.b * l.intensity, 0.0],
                            params: [l.distance, l.decay, 0.0, 0.0],
                        };
                        if l.cast_shadow && n_point == 0 && point_caster.is_none() {
                            let radius = if l.distance > 0.0 { l.distance } else { 50.0 };
                            point_caster = Some((p, radius, l.shadow));
                        }
                        n_point += 1;
                    }
                    Light::Spot(l) => {
                        if n_spot >= MAX_SPOT_LIGHTS { return; }
                        let p = obj.world_position();
                        let dir = transform_direction(&obj.matrix_world, l.direction).normalize();
                        let c = l.color;
                        let cos_outer = l.angle.cos();
                        let cos_inner = (l.angle * (1.0 - l.penumbra)).cos();
                        frame_u.spot_lights[n_spot] = SpotLightGpu {
                            position: [p.x, p.y, p.z, 0.0],
                            direction: [dir.x, dir.y, dir.z, 0.0],
                            color: [c.r * l.intensity, c.g * l.intensity, c.b * l.intensity, 0.0],
                            params: [l.distance, l.decay, cos_outer, cos_inner],
                        };
                        if l.cast_shadow && n_spot == 0 && spot_caster.is_none() {
                            spot_caster = Some((p, dir, l.angle, l.shadow));
                        }
                        n_spot += 1;
                    }
                    Light::Hemisphere(l) => {
                        if n_hemi >= MAX_HEMI_LIGHTS { return; }
                        let up = transform_direction(&obj.matrix_world, Vector3::UP).normalize();
                        frame_u.hemi_lights[n_hemi] = HemiLightGpu {
                            sky_color: [
                                l.sky_color.r * l.intensity,
                                l.sky_color.g * l.intensity,
                                l.sky_color.b * l.intensity,
                                0.0,
                            ],
                            ground_color: [
                                l.ground_color.r * l.intensity,
                                l.ground_color.g * l.intensity,
                                l.ground_color.b * l.intensity,
                                0.0,
                            ],
                            direction: [up.x, up.y, up.z, 0.0],
                        };
                        n_hemi += 1;
                    }
                    Light::RectArea(_) => { /* TODO LTC */ }
                },
                ObjectKind::Mesh(mesh) => {
                    self.ensure_geometry(&mesh.geometry);
                    draws.push(make_draw(&mesh.geometry, &mesh.material, obj, Topology::Triangle));
                }
                ObjectKind::LineSegments(ls) => {
                    self.ensure_geometry(&ls.geometry);
                    draws.push(make_draw(&ls.geometry, &ls.material, obj, Topology::Line));
                }
                ObjectKind::Points(p) => {
                    // three.js's PointsMaterial.size is a screen-space pixel
                    // size. WebGPU's PointList topology can't honor that —
                    // it always renders 1 fragment per point. We instead emit
                    // one camera-facing sprite-style draw per point, sized to
                    // match three.js's pixel-space convention.
                    let positions = p.geometry.get_attribute("position");
                    let size = match &*p.material {
                        crate::Material::Points(pm) => pm.size,
                        _ => 1.0,
                    };
                    let mat = &*p.material;
                    let nm3 = Matrix3::normal_matrix(&obj.matrix_world);
                    let c = mat.color();
                    if let Some(positions) = positions {
                        // three.js renders points as `size`-pixel squares via
                        // `gl_PointSize`. WebGPU has no per-vertex point size, so
                        // we emit a sprite quad. The sprite vertex shader scales
                        // by the model matrix's basis lengths and applies the
                        // perspective projection — so for the right at-distance
                        // size we work backwards from the desired NDC width.
                        //
                        // For a perspective camera, an object at view-z `vz` has
                        // a world-space width of `2 * vz / proj[0][0]` per unit
                        // of NDC. We want a sprite of `size` pixels wide on
                        // an 800-wide canvas (= size/400 NDC), so the model
                        // matrix's X basis length should be:
                        //   sx = (size / 400) * vz / proj[0][0]
                        let viewport_h = 600.0_f32;
                        let proj00 = proj_m.elements[0];
                        let canvas_w = 800.0_f32;
                        let _ = viewport_h;
                        let cnt = positions.count();
                        for vi in 0..cnt {
                            let px = positions.array[vi * 3] as f32;
                            let py = positions.array[vi * 3 + 1] as f32;
                            let pz = positions.array[vi * 3 + 2] as f32;
                            let m_local = crate::math::Matrix4::translation(crate::math::Vector3::new(px, py, pz));
                            let mut model = obj.matrix_world.multiply(&m_local);
                            // Project the point centre to view space to get its z, then
                            // size the quad to match the requested pixel size at that depth.
                            let wp = crate::math::Vector3::new(model.elements[12], model.elements[13], model.elements[14]);
                            let vz = (view_m.elements[2] * wp.x + view_m.elements[6] * wp.y
                                + view_m.elements[10] * wp.z + view_m.elements[14]).abs().max(0.0001);
                            let half_ndc = (size / canvas_w) * vz / proj00;
                            // Sprite vs uses `in.position * sx`; the unit quad has corners
                            // at ±0.5, so we want sx == size_in_world. Multiply by 2 because
                            // pixel size means total quad width, not half.
                            let sx = half_ndc * 2.0;
                            let mut e = model.elements;
                            for k in 0..3 { e[k    ] *= sx; }
                            for k in 0..3 { e[4 + k] *= sx; }
                            model.elements = e;
                            let world_pos = crate::math::Vector3::new(e[12], e[13], e[14]);
                            let svz = view_m.elements[2] * world_pos.x + view_m.elements[6] * world_pos.y
                                + view_m.elements[10] * world_pos.z + view_m.elements[14];
                            draws.push(DrawMesh {
                                key: std::ptr::null::<BufferGeometry>(),
                                topology: Topology::Sprite,
                                view_z: svz,
                                render_order: obj.render_order,
                                cast_shadow: false,
                                alpha_test: 0.0,
                                model: model.elements,
                                normal_matrix: mat3_to_mat4_array(&nm3),
                                color: [c.r, c.g, c.b, mat.opacity()],
                                emissive: [0.0; 4],
                                specular: [0.0; 4],
                                shininess_or_near: 0.0,
                                opacity_or_far: mat.opacity(),
                                roughness: 1.0,
                                metalness: 0.0,
                                ao_intensity: 1.0,
                                normal_scale: [1.0, 1.0],
                                toon_steps: 0,
                                physical: [0.0; 4],
                                shader_flags: 0,
                                kind: crate::materials::MaterialKind::Basic,
                                slots: mat.texture_slots(),
                                instance_buf_idx: usize::MAX,
                                instance_count: 1,
                                skin_idx: usize::MAX,
                            });
                        }
                    }
                }
                ObjectKind::Sprite(sprite) => {
                    // Inline construction — the sprite quad is built-in, so we
                    // can't reuse make_draw (which keys off an Arc<BufferGeometry>).
                    let mat = &*sprite.material;
                    let nm3 = Matrix3::normal_matrix(&obj.matrix_world);
                    let c = mat.color();
                    let sp = obj.world_position();
                    let svz = view_m.elements[2] * sp.x + view_m.elements[6] * sp.y
                        + view_m.elements[10] * sp.z + view_m.elements[14];
                    draws.push(DrawMesh {
                        key: std::ptr::null::<BufferGeometry>(),
                        topology: Topology::Sprite,
                        view_z: svz,
                        render_order: obj.render_order,
                        cast_shadow: false,
                        alpha_test: 0.0,
                        model: obj.matrix_world.elements,
                        normal_matrix: mat3_to_mat4_array(&nm3),
                        color: [c.r, c.g, c.b, mat.opacity()],
                        emissive: [0.0; 4],
                        specular: [0.0; 4],
                        shininess_or_near: 0.0,
                        opacity_or_far: mat.opacity(),
                        roughness: 1.0,
                        metalness: 0.0,
                        ao_intensity: 1.0,
                        normal_scale: [1.0, 1.0],
                        toon_steps: 0,
                        physical: [0.0; 4],
                        shader_flags: 0,
                        kind: mat.kind(),
                        slots: mat.texture_slots(),
                        instance_buf_idx: usize::MAX,
                        instance_count: 1,
                        skin_idx: usize::MAX,
                    });
                }
                ObjectKind::InstancedMesh(im) => {
                    self.ensure_geometry(&im.geometry);
                    if im.transforms.is_empty() { return; }
                    let mut matrices: Vec<f32> = Vec::with_capacity(im.transforms.len() * 16);
                    for t in &im.transforms { matrices.extend_from_slice(&t.elements); }
                    let buf = wgpu::util::DeviceExt::create_buffer_init(
                        &*self.device,
                        &wgpu::util::BufferInitDescriptor {
                            label: Some("threers instance buffer"),
                            contents: bytemuck::cast_slice(&matrices),
                            usage: wgpu::BufferUsages::VERTEX,
                        },
                    );
                    let idx = instance_buffers.len();
                    instance_buffers.push(buf);
                    let mut d = make_draw(&im.geometry, &im.material, obj, Topology::Instanced);
                    d.instance_buf_idx = idx;
                    d.instance_count = im.transforms.len() as u32;
                    draws.push(d);
                }
                ObjectKind::SkinnedMesh(sm) => {
                    self.ensure_geometry(&sm.geometry);
                    let key = geom_cache_key(&sm.geometry);
                    // Upload joints+weights to slot-1 vertex buffer (cached per-geometry).
                    if !self.skin_attr_cache.contains_key(&key) {
                        let joints = sm.geometry.get_attribute("joint");
                        let weights = sm.geometry.get_attribute("weight");
                        let (Some(j), Some(w)) = (joints, weights) else {
                            // Geometry isn't skin-equipped; draw at rest.
                            draws.push(make_draw(&sm.geometry, &sm.material, obj, Topology::Triangle));
                            return;
                        };
                        let count = j.count();
                        let mut interleaved: Vec<f32> = Vec::with_capacity(count * 8);
                        for i in 0..count {
                            interleaved.extend_from_slice(&j.array[i * 4..i * 4 + 4]);
                            interleaved.extend_from_slice(&w.array[i * 4..i * 4 + 4]);
                        }
                        let buf = wgpu::util::DeviceExt::create_buffer_init(
                            &*self.device,
                            &wgpu::util::BufferInitDescriptor {
                                label: Some("threers skin attrs"),
                                contents: bytemuck::cast_slice(&interleaved),
                                usage: wgpu::BufferUsages::VERTEX,
                            },
                        );
                        self.skin_attr_cache.insert(key, buf);
                    }
                    // Upload bone matrices (per-frame, per-skinned-mesh).
                    let mut bone_data: Vec<f32> = Vec::with_capacity(sm.skeleton.bone_matrices.len() * 16);
                    if sm.skeleton.bone_matrices.is_empty() {
                        // Identity fallback to keep storage buffer non-empty.
                        let id = crate::math::Matrix4::identity();
                        bone_data.extend_from_slice(&id.elements);
                    } else {
                        for m in &sm.skeleton.bone_matrices {
                            bone_data.extend_from_slice(&m.elements);
                        }
                    }
                    let bone_buf = wgpu::util::DeviceExt::create_buffer_init(
                        &*self.device,
                        &wgpu::util::BufferInitDescriptor {
                            label: Some("threers bone matrices"),
                            contents: bytemuck::cast_slice(&bone_data),
                            usage: wgpu::BufferUsages::STORAGE,
                        },
                    );
                    // Defer creating the combined mesh+skin bind group until
                    // after per_mesh is populated below — we need the per-mesh
                    // uniform buffer at binding 0.
                    let idx = skin_data.len();
                    skin_data.push(SkinData { bone_buf, bg: None });
                    let mut d = make_draw(&sm.geometry, &sm.material, obj, Topology::Skinned);
                    d.skin_idx = idx;
                    draws.push(d);
                }
                ObjectKind::Group => {}
            }
        });

        // Sort: opaque first (stable order), then transparents back-to-front so
        // alpha blending composites correctly. Sprites are already alpha-blended
        // and we treat them as transparent for ordering purposes.
        fn ord_class(t: Topology) -> u8 {
            match t {
                Topology::TriangleAlpha | Topology::Sprite => 1,
                _ => 0,
            }
        }
        draws.sort_by(|a, b| {
            ord_class(a.topology).cmp(&ord_class(b.topology))
                .then(a.render_order.cmp(&b.render_order))
                .then_with(|| {
                // For transparents we want farther-from-camera first. view_z is
                // negative for objects in front of the camera; a smaller (more
                // negative) value means farther away.
                if ord_class(a.topology) == 1 {
                    a.view_z.partial_cmp(&b.view_z).unwrap_or(std::cmp::Ordering::Equal)
                } else {
                    std::cmp::Ordering::Equal
                }
            })
        });

        frame_u.ambient = [ambient[0], ambient[1], ambient[2], 0.0];
        frame_u.light_counts = [n_dir as u32, n_point as u32, n_spot as u32, n_hemi as u32];

        // Shadow VP: orthographic from the caster light's direction. Camera fits
        // a fixed cube centered on the world origin (matches the default
        // ShadowSettings.camera_size). For a richer integration the box would
        // track the visible meshes' AABB.
        if let Some((dir, settings)) = shadow_caster {
            let s = settings.camera_size;
            let light_eye = -dir.normalize() * (s * 1.5);
            let light_view = Matrix4::look_at(light_eye, Vector3::ZERO, Vector3::UP);
            let light_proj = Matrix4::orthographic(-s, s, s, -s, settings.camera_near, settings.camera_far);
            let light_vp = light_proj.multiply(&light_view);
            frame_u.shadow_vp = light_vp.elements;
            frame_u.shadow_params[0] = 1.0;
            frame_u.shadow_params[1] = settings.bias.max(0.0);
        }
        if let Some((pos, radius, _settings)) = point_caster {
            frame_u.point_shadow_pos = [pos.x, pos.y, pos.z, radius];
        }
        if let Some((pos, dir, angle, settings)) = spot_caster {
            let target = pos + dir.normalize();
            let light_view = Matrix4::look_at(pos, target, Vector3::UP);
            // Perspective frustum: fov = 2 * outer_angle to cover the cone.
            let fov = (angle * 2.0).clamp(0.05, std::f32::consts::PI - 0.05);
            let light_proj = Matrix4::perspective(fov, 1.0, settings.camera_near, settings.camera_far);
            let light_vp = light_proj.multiply(&light_view);
            frame_u.spot_shadow_vp = light_vp.elements;
            frame_u.shadow_params[2] = 1.0;
            frame_u.shadow_params[3] = settings.bias.max(0.0);
        }

        // Sync environment cubemap. Upload on first sight, rebuild bind group on change.
        let env_key = scene.environment.as_ref().map(cube_cache_key);
        let env_rt_id = scene.environment_cube_rt;
        if env_key != self.env_cached_key || env_rt_id != self.env_cached_rt_id {
            let mut env_map_params = [0.0_f32; 4];
            if let Some(env) = &scene.environment {
                let k = cube_cache_key(env);
                if !self.cube_cache.contains_key(&k) {
                    let gc = GpuCubeTexture::upload(&self.device, &self.queue, env);
                    self.cube_cache.insert(k, gc);
                }
                if let Some(atlas) = &env.cube_uv_atlas {
                    env_map_params = [atlas.texel_width, atlas.texel_height, atlas.lod_max as f32, 1.0];
                }
            }
            frame_u.env_map_params = env_map_params;
            let cube_uv_view: &wgpu::TextureView = if let Some(env) = &scene.environment {
                self.cube_cache
                    .get(&cube_cache_key(env))
                    .and_then(|gc| gc.cube_uv_view.as_ref())
                    .unwrap_or(&self.default_cube_uv_view)
            } else {
                &self.default_cube_uv_view
            };
            let view: &wgpu::TextureView = if let Some(rt_id) = scene.environment_cube_rt {
                self.cube_rt_view_cache.get(&rt_id).unwrap_or(&self.default_env_cube.view)
            } else if let Some(env) = &scene.environment {
                &self.cube_cache[&cube_cache_key(env)].view
            } else {
                &self.default_env_cube.view
            };
            self.env_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("threers env bg"),
                layout: &self.env_bgl,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(view) },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&self.env_sampler) },
                    wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&self.shadow_view) },
                    wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::Sampler(&self.shadow_sampler) },
                    wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::TextureView(&self.spot_shadow_view) },
                    wgpu::BindGroupEntry { binding: 5, resource: wgpu::BindingResource::Sampler(&self.spot_shadow_sampler) },
                    wgpu::BindGroupEntry { binding: 6, resource: wgpu::BindingResource::TextureView(&self.point_shadow_cube_view) },
                    wgpu::BindGroupEntry { binding: 7, resource: wgpu::BindingResource::Sampler(&self.point_shadow_sampler) },
                    wgpu::BindGroupEntry { binding: 8, resource: wgpu::BindingResource::TextureView(cube_uv_view) },
                ],
            });
            self.env_cached_key = env_key;
            self.env_cached_rt_id = env_rt_id;
        }
        if let Some(env) = &scene.environment {
            if let Some(atlas) = &env.cube_uv_atlas {
                frame_u.env_map_params = [atlas.texel_width, atlas.texel_height, atlas.lod_max as f32, 1.0];
            }
        }

        // -- Ensure every texture referenced in this frame is uploaded. --
        for d in &draws {
            self.ensure_slot_uploaded(&d.slots.map);
            self.ensure_slot_uploaded(&d.slots.normal_map);
            self.ensure_slot_uploaded(&d.slots.roughness_map);
            self.ensure_slot_uploaded(&d.slots.metalness_map);
            self.ensure_slot_uploaded(&d.slots.ao_map);
            self.ensure_slot_uploaded(&d.slots.emissive_map);
            self.ensure_slot_uploaded(&d.slots.matcap_map);
        }

        // -- Ensure pool capacity. --
        while self.per_mesh.len() < draws.len() {
            let buf = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("threers mesh uniform"),
                size: std::mem::size_of::<MeshUniforms>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let mesh_bg = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("threers mesh bg"),
                layout: &self.mesh_bgl,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buf.as_entire_binding(),
                }],
            });
            // Texture bind group is rebuilt per-frame per-mesh below, but we
            // need a placeholder; we'll overwrite the pool entry in-place each
            // frame. Build a "default everything" tex bg for initialization.
            let tex_bg = self.make_default_tex_bg();
            self.per_mesh.push((buf, mesh_bg, tex_bg));
        }

        // -- Build combined mesh+skin bind groups now that per_mesh is grown. --
        for (i, d) in draws.iter().enumerate() {
            if d.topology == Topology::Skinned && d.skin_idx < skin_data.len() {
                let sd = &mut skin_data[d.skin_idx];
                let mesh_uniform_buf = &self.per_mesh[i].0;
                sd.bg = Some(self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("threers mesh skinned bg"),
                    layout: &self.mesh_skinned_bgl,
                    entries: &[
                        wgpu::BindGroupEntry { binding: 0, resource: mesh_uniform_buf.as_entire_binding() },
                        wgpu::BindGroupEntry { binding: 1, resource: sd.bone_buf.as_entire_binding() },
                    ],
                }));
            }
        }

        // -- Upload uniforms + rebuild per-mesh tex bind groups. --
        self.queue.write_buffer(&self.frame_buffer, 0, bytemuck::bytes_of(&frame_u));
        let mut new_tex_bgs: Vec<wgpu::BindGroup> = Vec::with_capacity(draws.len());
        for (i, d) in draws.iter().enumerate() {
            let mut flags: u32 = 0;
            if d.slots.map.is_some()           { flags |= FLAG_MAP; }
            if d.slots.normal_map.is_some()    { flags |= FLAG_NORMAL_MAP; }
            if d.slots.roughness_map.is_some() { flags |= FLAG_ROUGHNESS_MAP; }
            if d.slots.metalness_map.is_some() { flags |= FLAG_METALNESS_MAP; }
            if d.slots.ao_map.is_some()        { flags |= FLAG_AO_MAP; }
            if d.slots.emissive_map.is_some()  { flags |= FLAG_EMISSIVE_MAP; }
            if d.slots.matcap_map.is_some()    { flags |= FLAG_MATCAP_MAP; }

            let u = MeshUniforms {
                model: d.model,
                normal_matrix: d.normal_matrix,
                color: d.color,
                emissive: d.emissive,
                specular: d.specular,
                params: [d.shininess_or_near, d.opacity_or_far, d.roughness, d.metalness],
                params2: [
                    d.ao_intensity,
                    d.normal_scale[0],
                    d.normal_scale[1],
                    f32::from_bits(d.toon_steps),
                ],
                params3: d.physical,
                flags: [d.kind as u32, flags, d.shader_flags, d.alpha_test.to_bits()],
            };
            self.queue.write_buffer(&self.per_mesh[i].0, 0, bytemuck::bytes_of(&u));

            new_tex_bgs.push(self.build_tex_bg(&d.slots));
        }
        // Apply new tex bgs (separate loop to keep borrow rules happy).
        for (i, bg) in new_tex_bgs.into_iter().enumerate() {
            self.per_mesh[i].2 = bg;
        }

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("threers encoder"),
        });

        // Shadow depth pre-pass: render scene from the directional light's POV.
        if shadow_caster.is_some() {
            let mut spass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("threers shadow pass"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.shadow_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            spass.set_pipeline(&self.pipeline_shadow);
            spass.set_bind_group(0, &self.frame_bind_group, &[]);
            for (i, d) in draws.iter().enumerate() {
                if !d.cast_shadow { continue; }
                if d.topology != Topology::Triangle && d.topology != Topology::TriangleNoCull { continue; }
                let gm = &self.geom_cache[&d.key].mesh;
                spass.set_bind_group(1, &self.per_mesh[i].1, &[]);
                spass.set_bind_group(2, &self.per_mesh[i].2, &[]);
                spass.set_bind_group(3, &self.shadow_env_bind_group, &[]);
                spass.set_vertex_buffer(0, gm.vertex_buffer.slice(..));
                if let Some(ib) = &gm.index_buffer {
                    spass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
                    spass.draw_indexed(0..gm.index_count, 0, 0..1);
                } else {
                    spass.draw(0..gm.vertex_count, 0..1);
                }
            }
        }

        // Spot shadow depth pre-pass.
        if spot_caster.is_some() {
            let mut spass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("threers spot shadow pass"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.spot_shadow_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            spass.set_pipeline(&self.pipeline_shadow_spot);
            spass.set_bind_group(0, &self.frame_bind_group, &[]);
            for (i, d) in draws.iter().enumerate() {
                if !d.cast_shadow { continue; }
                if d.topology != Topology::Triangle && d.topology != Topology::TriangleNoCull { continue; }
                let gm = &self.geom_cache[&d.key].mesh;
                spass.set_bind_group(1, &self.per_mesh[i].1, &[]);
                spass.set_bind_group(2, &self.per_mesh[i].2, &[]);
                spass.set_bind_group(3, &self.shadow_env_bind_group, &[]);
                spass.set_vertex_buffer(0, gm.vertex_buffer.slice(..));
                if let Some(ib) = &gm.index_buffer {
                    spass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
                    spass.draw_indexed(0..gm.index_count, 0, 0..1);
                } else {
                    spass.draw(0..gm.vertex_count, 0..1);
                }
            }
        }

        // Point-light cubemap depth: 6 face passes, one per cube face.
        if let Some((pos, radius, _)) = point_caster {
            self.queue.submit(Some(encoder.finish()));
            encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("threers point shadow encoder"),
            });
            let face_lookat: [(Vector3, Vector3); 6] = [
                (Vector3::new( 1.0,  0.0,  0.0), Vector3::new(0.0, -1.0,  0.0)),
                (Vector3::new(-1.0,  0.0,  0.0), Vector3::new(0.0, -1.0,  0.0)),
                (Vector3::new( 0.0,  1.0,  0.0), Vector3::new(0.0,  0.0,  1.0)),
                (Vector3::new( 0.0, -1.0,  0.0), Vector3::new(0.0,  0.0, -1.0)),
                (Vector3::new( 0.0,  0.0,  1.0), Vector3::new(0.0, -1.0,  0.0)),
                (Vector3::new( 0.0,  0.0, -1.0), Vector3::new(0.0, -1.0,  0.0)),
            ];
            let fov = std::f32::consts::FRAC_PI_2;
            let proj = Matrix4::perspective(fov, 1.0, 0.1, radius.max(1.0));
            for face in 0..6 {
                let (forward, up) = face_lookat[face];
                let target = pos + forward;
                let view = Matrix4::look_at(pos, target, up);
                let vp = proj.multiply(&view);
                frame_u.cube_face_vp = vp.elements;
                self.queue.write_buffer(&self.frame_buffer, 0, bytemuck::bytes_of(&frame_u));

                let mut spass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("threers point shadow face pass"),
                    color_attachments: &[],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &self.point_shadow_face_views[face],
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
                spass.set_pipeline(&self.pipeline_shadow_point);
                spass.set_bind_group(0, &self.frame_bind_group, &[]);
                for (i, d) in draws.iter().enumerate() {
                    if d.topology != Topology::Triangle && d.topology != Topology::TriangleNoCull { continue; }
                    let gm = &self.geom_cache[&d.key].mesh;
                    spass.set_bind_group(1, &self.per_mesh[i].1, &[]);
                    spass.set_bind_group(2, &self.per_mesh[i].2, &[]);
                    spass.set_bind_group(3, &self.shadow_env_bind_group, &[]);
                    spass.set_vertex_buffer(0, gm.vertex_buffer.slice(..));
                    if let Some(ib) = &gm.index_buffer {
                        spass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
                        spass.draw_indexed(0..gm.index_count, 0, 0..1);
                    } else {
                        spass.draw(0..gm.vertex_count, 0..1);
                    }
                }
            }
        }

        {
            let bg_color = scene.background;
            // HalfFloat postfx RTs store linear light; sRGB canvas clears use raw bytes
            // (matches three.js gl.clearColor on an sRGB default framebuffer).
            let clear = if linear_framebuffer {
                wgpu::Color {
                    r: bg_color.r.powf(2.2) as f64,
                    g: bg_color.g.powf(2.2) as f64,
                    b: bg_color.b.powf(2.2) as f64,
                    a: 1.0,
                }
            } else {
                wgpu::Color {
                    r: bg_color.r as f64,
                    g: bg_color.g as f64,
                    b: bg_color.b as f64,
                    a: 1.0,
                }
            };
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("threers main pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(clear),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            pass.set_bind_group(0, &self.frame_bind_group, &[]);
            pass.set_bind_group(3, &self.env_bind_group, &[]);
            let mut last_topology: Option<Topology> = None;
            for (i, d) in draws.iter().enumerate() {
                if last_topology != Some(d.topology) {
                    let pipe = if linear_framebuffer {
                        match d.topology {
                            Topology::Triangle      => &self.pipeline_tri_f16,
                            Topology::TriangleAlpha => &self.pipeline_tri_alpha_f16,
                            Topology::TriangleNoCull => &self.pipeline_tri_nocull_f16,
                            Topology::Sky           => &self.pipeline_tri_sky_f16,
                            Topology::TriangleWire  => &self.pipeline_tri_wire_f16,
                            Topology::Line          => &self.pipeline_line_f16,
                            Topology::Point         => &self.pipeline_point_f16,
                            Topology::Sprite        => &self.pipeline_sprite_f16,
                            Topology::Instanced     => &self.pipeline_instanced_f16,
                            Topology::Skinned       => &self.pipeline_skinned_f16,
                        }
                    } else {
                        match d.topology {
                            Topology::Triangle      => &self.pipeline_tri,
                            Topology::TriangleAlpha => &self.pipeline_tri_alpha,
                            Topology::TriangleNoCull => &self.pipeline_tri_nocull,
                            Topology::Sky           => &self.pipeline_tri_sky,
                            Topology::TriangleWire  => &self.pipeline_tri_wire,
                            Topology::Line          => &self.pipeline_line,
                            Topology::Point         => &self.pipeline_point,
                            Topology::Sprite        => &self.pipeline_sprite,
                            Topology::Instanced     => &self.pipeline_instanced,
                            Topology::Skinned       => &self.pipeline_skinned,
                        }
                    };
                    pass.set_pipeline(pipe);
                    last_topology = Some(d.topology);
                }
                let gm = if d.topology == Topology::Sprite {
                    &self.sprite_quad
                } else {
                    &self.geom_cache[&d.key].mesh
                };
                if d.topology == Topology::Skinned {
                    // Skinned pipeline uses the combined mesh+skin BGL at group(1).
                    if let Some(bg) = skin_data.get(d.skin_idx).and_then(|s| s.bg.as_ref()) {
                        pass.set_bind_group(1, bg, &[]);
                    }
                    pass.set_bind_group(2, &self.per_mesh[i].2, &[]);
                    if let Some(skin_attrs) = self.skin_attr_cache.get(&d.key) {
                        pass.set_vertex_buffer(1, skin_attrs.slice(..));
                    }
                } else {
                    pass.set_bind_group(1, &self.per_mesh[i].1, &[]);
                    pass.set_bind_group(2, &self.per_mesh[i].2, &[]);
                }
                pass.set_vertex_buffer(0, gm.vertex_buffer.slice(..));
                let instance_count = if d.topology == Topology::Instanced {
                    pass.set_vertex_buffer(1, instance_buffers[d.instance_buf_idx].slice(..));
                    d.instance_count
                } else { 1 };
                // For TriangleWire we draw the wireframe line-list index buffer
                // (three.js WebGLGeometries: a,b,b,c,c,a per triangle).
                let (ib_opt, idx_count) = if d.topology == Topology::TriangleWire {
                    (gm.wire_index_buffer.as_ref(), gm.wire_index_count)
                } else {
                    (gm.index_buffer.as_ref(), gm.index_count)
                };
                if let Some(ib) = ib_opt {
                    pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
                    pass.draw_indexed(0..idx_count, 0, 0..instance_count);
                } else {
                    pass.draw(0..gm.vertex_count, 0..instance_count);
                }
            }
        }

        self.queue.submit(Some(encoder.finish()));
        let _ = (&self.frame_bgl,);
    }

    fn ensure_slot_uploaded(&mut self, slot: &Option<Arc<Texture>>) {
        let Some(t) = slot else { return; };
        // Skip upload for render-target-backed textures — the view lives in
        // the RenderTarget itself.
        if t.external_rt_id.is_some() { return; }
        let key = tex_cache_key(t);
        if !self.tex_cache.contains_key(&key) {
            let gt = GpuTexture::upload(&self.device, &self.queue, t);
            self.tex_cache.insert(key, gt);
        }
    }

    fn view_for<'a>(&'a self, slot: &Option<Arc<Texture>>, default: &'a GpuTexture) -> &'a wgpu::TextureView {
        match slot {
            Some(t) => {
                // If this Texture is RT-backed, sample from the RT's color view.
                if let Some(rt_id) = t.external_rt_id {
                    if let Some(view) = self.rt_view_cache.get(&rt_id) {
                        return view;
                    }
                }
                &self.tex_cache[&tex_cache_key(t)].view
            }
            None => &default.view,
        }
    }

    /// Borrow an RT's sample-side view by id. Caller must have previously
    /// `register_render_target`'d this id. Used by post-fx to bind the input.
    pub fn rt_view_cache_get(&self, rt_id: u32) -> &wgpu::TextureView {
        self.rt_view_cache.get(&rt_id).expect("rt id not registered")
    }

    /// Register a CubeRenderTarget so scene.environment_cube_rt can sample
    /// from its 6-face cube view as an env map.
    pub fn register_cube_render_target(&mut self, rt_id: u32, rt: &Arc<super::CubeRenderTarget>) {
        let sample_format = rt.format.add_srgb_suffix();
        let view = rt.color_texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("threers cube rt sample view"),
            format: Some(sample_format),
            dimension: Some(wgpu::TextureViewDimension::Cube),
            base_array_layer: 0,
            array_layer_count: Some(6),
            base_mip_level: 0,
            mip_level_count: Some(1),
            aspect: wgpu::TextureAspect::All,
        });
        self.cube_rt_view_cache.insert(rt_id, view);
    }

    /// Register a RenderTarget so RT-backed Textures can sample from its color view.
    /// Creates a sample-side view using the sRGB-variant of the storage format
    /// (when applicable) so the hardware does sRGB→linear decode on read,
    /// matching three.js's WebGL behavior where RT textures auto-decode.
    pub fn register_render_target(&mut self, rt_id: u32, rt: &Arc<super::RenderTarget>) {
        let sample_format = rt_sample_format(rt.format);
        let view = rt.color_texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("threers rt sample view"),
            format: Some(sample_format),
            ..Default::default()
        });
        self.rt_view_cache.insert(rt_id, view);
        let linear_view = rt.color_texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("threers rt linear sample view"),
            format: Some(rt.format),
            ..Default::default()
        });
        self.rt_linear_view_cache.insert(rt_id, linear_view);
        let depth_view = rt.depth_texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("threers rt depth sample view"),
            aspect: wgpu::TextureAspect::DepthOnly,
            ..Default::default()
        });
        self.rt_depth_view_cache.insert(rt_id, depth_view);
        self.rt_registry.insert(rt_id, Arc::clone(rt));
    }

    fn build_tex_bg(&self, slots: &MaterialTextureSlots) -> wgpu::BindGroup {
        let albedo = self.view_for(&slots.map, &self.default_white_srgb);
        let normal = self.view_for(&slots.normal_map, &self.default_normal);
        let roughness = self.view_for(&slots.roughness_map, &self.default_white_linear);
        let metalness = self.view_for(&slots.metalness_map, &self.default_white_linear);
        let ao = self.view_for(&slots.ao_map, &self.default_white_linear);
        let emissive = self.view_for(&slots.emissive_map, &self.default_white_srgb);
        let matcap = self.view_for(&slots.matcap_map, &self.default_white_srgb);

        // Pick sampler based on the primary texture's filter/wrap. We use
        // `slots.map` as the canonical choice (matcap uses `matcap_map`).
        let chosen = slots.map.as_ref().or(slots.matcap_map.as_ref());
        let sampler = match chosen {
            Some(t) => self.pick_sampler(t),
            None    => &self.sampler_linear,
        };

        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("threers tex bg"),
            layout: &self.tex_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(albedo) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(normal) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(roughness) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(metalness) },
                wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::TextureView(ao) },
                wgpu::BindGroupEntry { binding: 5, resource: wgpu::BindingResource::TextureView(emissive) },
                wgpu::BindGroupEntry { binding: 6, resource: wgpu::BindingResource::TextureView(matcap) },
                wgpu::BindGroupEntry { binding: 7, resource: wgpu::BindingResource::Sampler(sampler) },
            ],
        })
    }

    /// Select one of the 4 pre-built samplers based on a Texture's filter/wrap
    /// fields. Mirrors three.js: NearestFilter→nearest, LinearFilter→linear;
    /// RepeatWrapping→repeat, ClampToEdgeWrapping→clamp.
    fn pick_sampler<'a>(&'a self, t: &Texture) -> &'a wgpu::Sampler {
        use crate::textures::{TextureFilter, TextureWrap};
        let nearest = matches!(t.mag_filter, TextureFilter::Nearest);
        let repeat  = matches!(t.wrap_s, TextureWrap::Repeat) || matches!(t.wrap_t, TextureWrap::Repeat);
        match (nearest, repeat) {
            (false, false) => &self.sampler_linear,
            (true,  false) => &self.sampler_nearest_clamp,
            (false, true)  => &self.sampler_linear_repeat,
            (true,  true)  => &self.sampler_nearest_repeat,
        }
    }

    fn make_default_tex_bg(&self) -> wgpu::BindGroup {
        self.build_tex_bg(&MaterialTextureSlots::default())
    }
}

fn create_depth_view(device: &wgpu::Device, width: u32, height: u32) -> wgpu::TextureView {
    let tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("threers depth"),
        size: wgpu::Extent3d { width: width.max(1), height: height.max(1), depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    tex.create_view(&wgpu::TextureViewDescriptor::default())
}

fn transform_direction(m: &Matrix4, v: Vector3) -> Vector3 {
    let e = &m.elements;
    Vector3::new(
        e[0] * v.x + e[4] * v.y + e[8]  * v.z,
        e[1] * v.x + e[5] * v.y + e[9]  * v.z,
        e[2] * v.x + e[6] * v.y + e[10] * v.z,
    )
}

fn mat3_to_mat4_array(m: &Matrix3) -> [f32; 16] {
    let e = &m.elements;
    [
        e[0], e[1], e[2], 0.0,
        e[3], e[4], e[5], 0.0,
        e[6], e[7], e[8], 0.0,
        0.0,  0.0,  0.0,  1.0,
    ]
}
