//! wgpu rendering: scene traversal, material pipelines, shadows, and post-fx.
//!
//! [`Renderer`] is the main entry point on native. It owns GPU pipelines for
//! each topology (triangles, lines, points, sprites, skinned meshes), uploads
//! geometry and textures through internal caches, and exposes [`Renderer::apply_postfx`]
//! for fullscreen effects used by both native [`crate::postprocessing`] and the
//! wasm [`crate::wasm::WebRenderer`] path.
//!
//! [`RenderTarget`] / [`CubeRenderTarget`] are offscreen color (+ depth) buffers
//! compatible with three.js `WebGLRenderTarget` usage in the JS shim.

mod shader;
mod gpu_mesh;
pub mod gpu_texture;
mod render_target;
mod renderer;

pub use renderer::{Renderer, PostFxCamera};
pub use render_target::{RenderTarget, CubeRenderTarget};
pub use gpu_texture::GpuCubeTexture;
