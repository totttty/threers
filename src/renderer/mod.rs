//! wgpu rendering: scene traversal, material pipelines, shadows, and post-fx.
//!
//! [`Renderer`] is the main entry point on native. It owns GPU pipelines for
//! each topology (triangles, lines, points, sprites, skinned meshes), uploads
//! geometry and textures through internal caches, and exposes [`Renderer::apply_postfx`]
//! for fullscreen effects used by both native [`crate::postprocessing`] and the
//! wasm `WebRenderer` path.
//!
//! [`RenderTarget`] / [`CubeRenderTarget`] are offscreen color (+ depth) buffers
//! compatible with three.js `WebGLRenderTarget` usage in the JS shim.
//!
//! On native targets, [`crate::renderer::headless`] provides [`HeadlessRenderer`] for
//! offscreen RGBA readback without managing a window surface.

mod gpu_mesh;
pub mod gpu_texture;
mod render_target;
mod renderer;
mod shader;

// Batteries-included headless offscreen renderer (native-only: pollster).
#[cfg(not(target_arch = "wasm32"))]
pub mod headless;

pub use gpu_texture::GpuCubeTexture;
#[cfg(not(target_arch = "wasm32"))]
pub use headless::{HeadlessBuilder, HeadlessConfig, HeadlessRenderer};
pub use render_target::{CubeRenderTarget, RenderTarget};
pub use renderer::{PostFxCamera, Renderer, LIGHT_PROBE_COEFFICIENT_BYTES};
pub use crate::light_probes::MAX_LIGHT_PROBE_GRID_PROBES;
