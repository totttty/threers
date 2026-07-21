//! Texture sources. CPU-side: own the pixel data (or a reference to it) plus
//! sampler/format metadata. The renderer uploads them on demand and caches the
//! resulting `wgpu::Texture` by `Arc` pointer identity.

mod cube_texture;
mod data_texture;
mod depth_texture;
mod texture;

pub use cube_texture::{CubeTexture, CubeUvAtlas};
pub use data_texture::DataTexture;
pub use depth_texture::DepthTexture;
pub use texture::{Texture, TextureFilter, TextureFormat, TextureWrap};
