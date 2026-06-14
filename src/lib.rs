//! threers — a three.js-inspired 3D library for Rust.
//!
//! Mirrors three.js's core architecture: a scene graph of [`Object3D`]s,
//! [`BufferGeometry`] with named attributes, [`Material`]s, [`Camera`]s, and a
//! [`Renderer`] that walks the graph and draws. Backed by wgpu on native and
//! WebGPU in the browser.
//!
//! # Crates and targets
//!
//! - **`rlib`**: native desktop / tooling (`cargo run --example cube`)
//! - **`cdylib`**: wasm32 WebAssembly (`wasm-pack build`)
//!
//! # Web usage
//!
//! The [`wasm`] module exposes `#[wasm_bindgen]` types. Browser apps import
//! `web/threejs-shim.js`, which re-exports them as `THREE.*`.
//!
//! # Modules
//!
//! | Module | Role |
//! |--------|------|
//! | [`core`] | Scene graph, geometry, raycaster |
//! | [`renderer`] | wgpu draw + post-processing |
//! | [`postprocessing`] | EffectComposer-style pass chain (native) |
//! | [`loaders`] | glTF, OBJ, HDR, … |
//! | [`extras`] | PMREM, noise, marching cubes, … |

pub mod math;
pub mod core;
pub mod geometries;
pub mod materials;
pub mod lights;
pub mod textures;
pub mod curves;
pub mod cameras;
pub mod controls;
pub mod animation;
pub mod loaders;
pub mod audio;
pub mod postprocessing;
pub mod renderers;
pub mod utils;
pub mod helpers;
pub mod extras;
pub mod stats;
pub mod scene;

#[cfg(target_arch = "wasm32")]
pub mod wasm;
pub mod renderer;

#[cfg(feature = "mesh-bvh")]
pub mod mesh_bvh;

#[cfg(target_arch = "wasm32")]
#[macro_export]
macro_rules! log {
    ($($t:tt)*) => {{
        let s = format!($($t)*);
        web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&s));
    }};
}

#[cfg(not(target_arch = "wasm32"))]
#[macro_export]
macro_rules! log {
    ($($t:tt)*) => {{ eprintln!($($t)*); }};
}

pub use math::{
    Vector2, Vector3, Vector4,
    Matrix3, Matrix4,
    Quaternion, Euler, Color,
    Box2, Box3, Sphere, Ray, Plane, Triangle, Frustum,
    Spherical, Cylindrical, Line3,
};
pub use core::{
    Object3D, ObjectId, ObjectKind, ObjectArena,
    BufferGeometry, BufferAttribute, Mesh,
    Layers, Clock, Raycaster, Intersection,
    LineSegments, Points, Sprite, InstancedMesh,
    Bone, Skeleton, SkinnedMesh, MorphTarget, MorphAttributes,
};
pub use geometries::{
    BoxGeometry, PlaneGeometry, SphereGeometry,
    CircleGeometry, RingGeometry,
    CylinderGeometry, ConeGeometry,
    TorusGeometry, TorusKnotGeometry, CapsuleGeometry,
    PolyhedronGeometry, TetrahedronGeometry, OctahedronGeometry,
    IcosahedronGeometry, DodecahedronGeometry,
    EdgesGeometry, WireframeGeometry,
    LatheGeometry, TubeGeometry, ExtrudeGeometry,
    ParametricGeometry, ConvexGeometry, DecalGeometry,
    TextGeometry, Glyph, BoxLineGeometry,
};
pub use materials::{
    Material, MaterialKind, MaterialTextureSlots,
    BasicMaterial, LambertMaterial, PhongMaterial,
    StandardMaterial, PhysicalMaterial,
    NormalMaterial, DepthMaterial, ToonMaterial, MatcapMaterial,
    LineBasicMaterial, PointsMaterial, SpriteMaterial,
};
pub use materials::MirrorMaterial;
pub use lights::{
    Light, AmbientLight, DirectionalLight, PointLight,
    SpotLight, HemisphereLight, RectAreaLight, ShadowSettings,
};
pub use textures::{
    Texture, TextureFormat, TextureFilter, TextureWrap,
    CubeTexture, DataTexture, DepthTexture,
};
pub use helpers::{
    AxesHelper, GridHelper, BoxHelper, CameraHelper, ArrowHelper, PolarGridHelper,
    DirectionalLightHelper, PointLightHelper, SpotLightHelper,
    HemisphereLightHelper, SkeletonHelper,
    VertexNormalsHelper, VertexTangentsHelper,
};
pub use curves::{
    Curve2, Curve3,
    LineCurve, LineCurve3,
    QuadraticBezierCurve, QuadraticBezierCurve3,
    CubicBezierCurve, CubicBezierCurve3,
    EllipseCurve,
    CatmullRomCurve3, SplineCurve,
    CurvePath, Path, Shape,
    NURBSCurve, NURBSSurface,
};
pub use cameras::{Camera, PerspectiveCamera, OrthographicCamera};
pub use controls::{
    OrbitControls, TrackballControls, FirstPersonControls,
    DragControls, ArcballControls, PointerLockControls, PointerEvent,
};
pub use animation::{
    Interpolation, KeyframeTrack, TrackTarget,
    AnimationClip, AnimationMixer, AnimationAction,
};
pub use loaders::{
    GltfLoader, GltfScene, GltfError, GltfImages,
    ObjLoader, StlLoader, PlyLoader,
    HdrLoader, HdrError, ExrLoader, ExrError,
    FbxLoader, FbxError, ColladaLoader, ColladaError,
    TtfFont, TtfError, TtfGlyph,
};
pub use audio::{Audio, AudioListener, PositionalAudio, AudioAnalyser, AudioBackend, NoopBackend};
pub use postprocessing::{
    EffectComposer, Pass,
    RenderPass, BloomPass, FxaaPass, OutlinePass, ToneMappingPass,
    FilmPass, GlitchPass, SsaoPass, SsrPass, CopyPass,
};
pub use utils::{compute_vertex_normals, compute_tangents, merge_geometries, center as center_geometry, scale as scale_geometry};
pub use renderers::{Css2dRenderer, Css3dRenderer, SvgRenderer};
pub use extras::{MarchingCubes, CcdIkSolver, IkBone, Octree, SimplexNoise, PmremGenerator, PMREM_MIP_LEVELS};
pub use stats::Stats;
pub use scene::Scene;
pub use renderer::{Renderer, RenderTarget};

#[cfg(feature = "mesh-bvh")]
pub use mesh_bvh::{MeshBvh, BvhHit, BuildOptions as MeshBvhBuildOptions, CENTER, NOT_INTERSECTED, INTERSECTED, CONTAINED};
