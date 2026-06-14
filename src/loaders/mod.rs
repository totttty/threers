//! Asset loaders. Hand-written parsers, zero binary-crate deps.

mod json;
mod gltf;
mod obj;
mod stl;
mod ply;
mod hdr;
mod exr;
mod fbx;
mod collada;
mod xml;
mod ttf;
pub mod deflate;

pub use gltf::{GltfLoader, GltfScene, GltfError, GltfImages, add_to_scene as gltf_add_to_scene};
pub use obj::ObjLoader;
pub use stl::StlLoader;
pub use ply::PlyLoader;
pub use hdr::{HdrLoader, HdrError};
pub use exr::{ExrLoader, ExrError};
pub use fbx::{FbxLoader, FbxError};
pub use collada::{ColladaLoader, ColladaError};
pub use ttf::{TtfFont, TtfError, TtfGlyph};
