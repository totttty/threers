use crate::math::{Color, Vector2};
use crate::textures::Texture;
use std::sync::Arc;

/// Extended PBR with clearcoat, IOR, transmission, sheen, iridescence layers.
/// Matches three.js's `MeshPhysicalMaterial`. The current renderer only honors
/// the base PBR terms; the extended layers are stored but ignored until the
/// extended BRDF lands in a follow-up phase.
#[derive(Debug, Clone)]
pub struct PhysicalMaterial {
    pub color: Color,
    pub emissive: Color,
    pub emissive_intensity: f32,
    pub roughness: f32,
    pub metalness: f32,
    pub ao_intensity: f32,
    pub normal_scale: Vector2,
    pub opacity: f32,
    pub wireframe: bool,

    pub clearcoat: f32,
    pub clearcoat_roughness: f32,
    pub ior: f32,
    pub transmission: f32,
    pub thickness: f32,
    /// Chromatic dispersion (Abbe-like). 0 = none. Splits the refraction IOR per
    /// RGB channel so the screen-space-glass path shows a rainbow edge fringe.
    pub dispersion: f32,
    /// Per-vertex emission: adds `vertexColor.rgb × vertexColor.a` as emitted
    /// light, scaled by this factor (0 = off). A general mechanism for data-baked
    /// or decal glow — the alpha channel gates *where* it emits and the rgb sets
    /// the color, both authored into the mesh's vertex colors by the app.
    pub vertex_emissive: f32,
    pub sheen: f32,
    pub sheen_color: Color,
    pub sheen_roughness: f32,
    pub iridescence: f32,
    pub iridescence_ior: f32,
    /// Anisotropy strength (0 = isotropic). Stretches the specular highlight
    /// along the surface tangent for a brushed/streaked look.
    pub anisotropy: f32,
    /// Anisotropy direction as a rotation (radians) of the tangent in the
    /// surface plane.
    pub anisotropy_rotation: f32,
    pub attenuation_distance: f32,
    pub attenuation_color: Color,

    pub map: Option<Arc<Texture>>,
    pub normal_map: Option<Arc<Texture>>,
    pub roughness_map: Option<Arc<Texture>>,
    pub metalness_map: Option<Arc<Texture>>,
    pub ao_map: Option<Arc<Texture>>,
    pub emissive_map: Option<Arc<Texture>>,

    pub side: u32,

    /// Transparency compositing technique (blend / single-layer glass / OIT).
    pub transparency: super::TransparencyMode,
}

impl Default for PhysicalMaterial {
    fn default() -> Self {
        Self {
            transparency: super::TransparencyMode::default(),
            color: Color::WHITE,
            emissive: Color::BLACK,
            emissive_intensity: 1.0,
            roughness: 1.0,
            metalness: 0.0,
            ao_intensity: 1.0,
            normal_scale: Vector2::new(1.0, 1.0),
            opacity: 1.0,
            wireframe: false,
            clearcoat: 0.0,
            clearcoat_roughness: 0.0,
            ior: 1.5,
            transmission: 0.0,
            thickness: 0.01,
            dispersion: 0.0,
            vertex_emissive: 0.0,
            sheen: 0.0,
            sheen_color: Color::BLACK,
            sheen_roughness: 1.0,
            iridescence: 0.0,
            iridescence_ior: 1.3,
            anisotropy: 0.0,
            anisotropy_rotation: 0.0,
            attenuation_distance: f32::INFINITY,
            attenuation_color: Color::WHITE,
            map: None,
            normal_map: None,
            roughness_map: None,
            metalness_map: None,
            ao_map: None,
            emissive_map: None,
            side: 0,
        }
    }
}

impl PhysicalMaterial {
    pub fn new(color: Color) -> Self {
        Self {
            color,
            ..Default::default()
        }
    }
}
