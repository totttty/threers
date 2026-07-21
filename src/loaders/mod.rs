//! Asset loaders. Hand-written parsers, zero binary-crate deps.

mod collada;
pub mod deflate;
mod exr;
mod fbx;
mod gltf;
mod hdr;
mod json;
mod obj;
mod ply;
mod stl;
mod ttf;
mod xml;

pub use collada::{ColladaError, ColladaLoader};
pub use exr::{ExrError, ExrLoader};
pub use fbx::{FbxError, FbxLoader};
pub use gltf::{add_to_scene as gltf_add_to_scene, GltfError, GltfImages, GltfLoader, GltfScene};
pub use hdr::{HdrError, HdrLoader};
pub use obj::ObjLoader;
pub use ply::PlyLoader;
pub use stl::StlLoader;
pub use ttf::{TtfError, TtfFont, TtfGlyph};
